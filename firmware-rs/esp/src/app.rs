//! Цикл заліза: кнопки, таймери, табло, buzzer, виклики черги.

use std::sync::Arc;

use esp_idf_svc::hal::gpio::{Input, PinDriver, Pull};
use esp_idf_svc::hal::i2c::{I2cConfig, I2cDriver, I2C0};
use esp_idf_svc::hal::ledc::{CHANNEL0, TIMER0};
use esp_idf_svc::hal::units::Hertz;
use log::{info, warn};
use shower_core::input::{DebouncedButton, Edge, RateLimit};
use shower_core::queue::{Event, Queue};
use shower_core::schedule::Mode;
use shower_core::shower::{Occupant, WaterTemperature};
use shower_core::texts::{self, oled as oled_text};

use crate::board::BoardPins;
use crate::config::SCHEDULE;
use crate::hw::buzzer::Buzzer;
use crate::hw::oled::Oled;
use crate::hw::tm1637::Tm1637Pair;
use crate::net;
use crate::outbox::Job;
use crate::shower::{Shower, Transition};
use crate::state::{Closed, Shared};

const TEMP_DEBOUNCE_MS: u64 = 30;
const TEMP_REFIRE_MS: u64 = 300;
const QUEUE_BTN_REFIRE_MS: u64 = 2000;
const FLASH_MS: u64 = 2000;
const TICKET_FLASH_MS: u64 = 5000;
const OLED_REFRESH_MS: u64 = 500;
const ANIM_FRAME_MS: u64 = 100;
const MODE_CHECK_MS: u64 = 1000;
const QUEUE_TICK_MS: u64 = 200;
const TIMER_BRIGHTNESS: u8 = 7;

struct TempButton {
    pin: PinDriver<'static, Input>,
    debounce: DebouncedButton,
}

pub struct App {
    shared: Arc<Shared>,
    showers: [Shower; 2],
    timers: Tm1637Pair,
    oled: Oled,
    buzzer: Buzzer,
    queue_button: PinDriver<'static, Input>,
    queue_edge: Edge,
    queue_rate: RateLimit,
    temp_buttons: [[Option<TempButton>; 4]; 2],
    mode: Mode,
    mode_checked_at: Option<u64>,
    queue_ticked_at: Option<u64>,
    /// Тимчасовий текст на OLED замість статусу: (рядки, до якого часу).
    flash: Option<([String; 2], u64)>,
    status_shown_at: Option<u64>,
    banner_x: i32,
    banner_frame_at: Option<u64>,
}

impl App {
    pub fn new(
        shared: Arc<Shared>,
        pins: BoardPins,
        i2c: I2C0<'static>,
        ledc_timer: TIMER0<'static>,
        ledc_channel: CHANNEL0<'static>,
    ) -> anyhow::Result<Self> {
        let [b1, b2] = pins.shower_button;
        let [r1, r2] = pins.red_led;
        let [g1, g2] = pins.green_led;
        let showers = [Shower::new(1, b1, r1, g1)?, Shower::new(2, b2, r2, g2)?];
        // Душ, зайнятий ще до старту, рахуємо від старту.
        for (i, s) in showers.iter().enumerate() {
            if s.is_busy() {
                shared.shower_occupied(i, 0);
            }
        }

        let mut timers = Tm1637Pair::new(pins.timer_clk, pins.timer_dio, TIMER_BRIGHTNESS)?;
        for i in 0..2 {
            timers.write(i, shower_core::segments::FREE)?;
        }

        let i2c = I2cDriver::new(i2c, pins.oled_sda, pins.oled_scl, &I2cConfig::new().baudrate(Hertz(400_000)))?;
        let oled = Oled::new(i2c);
        let buzzer = Buzzer::new(ledc_timer, ledc_channel, pins.buzzer)?;

        let queue_button = PinDriver::input(pins.queue_button, Pull::Up)?;
        let queue_edge = Edge::new(queue_button.is_low());

        let mut temp_buttons: [[Option<TempButton>; 4]; 2] = Default::default();
        for (s, row) in pins.temp_buttons.into_iter().enumerate() {
            for (c, pin) in row.into_iter().enumerate() {
                let Some(pin) = pin else {
                    info!("Init: S{} T{} вимкнено", s + 1, c + 1);
                    continue;
                };
                let pin = PinDriver::input(pin, Pull::Up)?;
                info!("Init: S{} T{} -> {}", s + 1, c + 1, if pin.is_high() { "HIGH (idle)" } else { "LOW (pressed?)" });
                temp_buttons[s][c] = Some(TempButton {
                    pin,
                    debounce: DebouncedButton::new(TEMP_DEBOUNCE_MS, TEMP_REFIRE_MS),
                });
            }
        }

        Ok(Self {
            shared,
            showers,
            timers,
            oled,
            buzzer,
            queue_button,
            queue_edge,
            queue_rate: RateLimit::new(QUEUE_BTN_REFIRE_MS),
            temp_buttons,
            mode: Mode::Open,
            mode_checked_at: None,
            queue_ticked_at: None,
            flash: None,
            status_shown_at: None,
            banner_x: 0,
            banner_frame_at: None,
        })
    }

    pub fn run(&mut self, now: u64) {
        self.refresh_mode(now);
        if let Some(is_error) = self.shared.take_beep() {
            self.buzzer.beep(is_error, now);
        }
        self.buzzer.tick(now);

        self.poll_temp_buttons(now);
        self.poll_showers(now);
        self.poll_queue_button(now);
        if self.mode == Mode::Open {
            self.tick_queue(now);
        }
        self.update_timers(now);
        self.update_oled(now);
    }

    fn refresh_mode(&mut self, now: u64) {
        if self.mode_checked_at.is_some_and(|t| now - t < MODE_CHECK_MS) {
            return;
        }
        self.mode_checked_at = Some(now);
        let mode = SCHEDULE.mode(net::local_hour());
        if mode == self.mode {
            return;
        }
        info!("Mode: {:?} -> {mode:?}", self.mode);
        match mode {
            Mode::Night { .. } => self.shared.modify(Queue::clear),
            Mode::Break { .. } => self.shared.modify(Queue::reset_calls),
            Mode::Open => {}
        }
        self.mode = mode;
        self.shared.set_mode(mode);
    }

    fn flash(&mut self, lines: [String; 2], duration_ms: u64, now: u64) {
        self.flash = Some((lines, now + duration_ms));
    }

    fn error(&mut self, message: &str, now: u64) {
        self.buzzer.beep(true, now);
        self.flash(oled_text::error(message), FLASH_MS, now);
    }

    fn poll_showers(&mut self, now: u64) {
        for i in 0..2 {
            match self.showers[i].poll(now) {
                Some(Transition::Occupied) => {
                    self.shared.shower_occupied(i, now);
                    // Поза робочим часом черга не рухається.
                    let entered = match self.mode {
                        Mode::Open => self.shared.modify(Queue::on_enter),
                        _ => None,
                    };
                    let occupant = entered.as_ref().map_or(Occupant::WalkIn, Occupant::from);
                    info!("Shower {}: зайшов {}", i + 1, occupant.display_name());
                    self.showers[i].occupant = Some(occupant);
                    self.buzzer.beep(false, now);
                }
                Some(Transition::Freed) => {
                    self.shared.shower_freed(i, now);
                    self.showers[i].occupant = None;
                }
                None => {}
            }
        }
    }

    fn tick_queue(&mut self, now: u64) {
        if self.queue_ticked_at.is_some_and(|t| now - t < QUEUE_TICK_MS) {
            return;
        }
        self.queue_ticked_at = Some(now);
        let free = self.showers.iter().filter(|s| !s.is_busy()).count();
        for event in self.shared.modify(|q| q.tick(free, now)) {
            info!("Queue: {event:?}");
            match event {
                Event::Called(member) => match member.user_id() {
                    Some(user_id) => self.shared.send(Job::Call { user_id, name: member.display_name() }),
                    // Талон кличе табло; пік, щоб у коридорі глянули на OLED.
                    None => self.buzzer.beep(false, now),
                },
                Event::Moved { member, position } => {
                    self.shared.send(Job::Notice(texts::timeout_moved(&member.display_name(), position)));
                }
                Event::Removed(member) => {
                    self.shared.send(Job::Notice(texts::timeout_removed(&member.display_name())));
                }
            }
        }
    }

    fn poll_queue_button(&mut self, now: u64) {
        if !self.queue_edge.rising(self.queue_button.is_low()) {
            return;
        }
        if !self.queue_rate.try_fire(now) {
            return self.error(oled_text::WAIT, now);
        }
        match self.shared.take_ticket() {
            Ok(Some(n)) => {
                info!("Queue: видано талон №{n}");
                self.buzzer.beep(false, now);
                self.flash(oled_text::ticket(n), TICKET_FLASH_MS, now);
            }
            Ok(None) => self.error(oled_text::QUEUE_FULL, now),
            Err(Closed) => self.error(oled_text::CLOSED, now),
        }
    }

    fn poll_temp_buttons(&mut self, now: u64) {
        for s in 0..2 {
            for c in 0..4 {
                let fired = match &mut self.temp_buttons[s][c] {
                    Some(b) => b.debounce.update(b.pin.is_low(), now),
                    None => false,
                };
                if fired {
                    self.on_temperature(s, c as u8 + 1, now);
                }
            }
        }
    }

    fn on_temperature(&mut self, shower: usize, level: u8, now: u64) {
        info!("Input: S{} T{level}", shower + 1);
        let Some(occupant) = self.showers[shower].occupant.clone() else {
            return self.error(&oled_text::no_occupant(shower + 1), now);
        };
        let water = WaterTemperature { level, occupant, time: net::local_time_string() };
        info!("Shower {}: T={level} {} о {}", shower + 1, water.occupant.display_name(), water.time);
        self.shared.set_water(shower, water);
        self.buzzer.beep(false, now);
        self.flash(oled_text::error(&oled_text::temperature_set(shower + 1, level)), FLASH_MS, now);
    }

    fn update_timers(&mut self, now: u64) {
        for (i, shower) in self.showers.iter_mut().enumerate() {
            let seg = shower.segments(now);
            if shower.shown == Some(seg) {
                continue;
            }
            match self.timers.write(i, seg) {
                Ok(()) => shower.shown = Some(seg),
                Err(e) => warn!("TM1637 #{}: {e}", i + 1),
            }
        }
    }

    fn update_oled(&mut self, now: u64) {
        if let Some((lines, until)) = &self.flash {
            if now < *until {
                self.oled.show_lines(lines.clone());
                return;
            }
            self.flash = None;
        }
        match self.mode {
            Mode::Night { .. } => self.animate_off_time(now),
            mode => {
                if self.status_shown_at.is_some_and(|t| now - t <= OLED_REFRESH_MS) {
                    return;
                }
                let lines = self.shared.read(|q| match mode {
                    Mode::Break { until_hour } => oled_text::break_status(until_hour, q.len()),
                    _ => oled_text::queue_status(q),
                });
                self.oled.show_lines(lines);
                self.status_shown_at = Some(now);
            }
        }
    }

    fn animate_off_time(&mut self, now: u64) {
        if self.banner_frame_at.is_some_and(|t| now - t < ANIM_FRAME_MS) {
            return;
        }
        self.banner_frame_at = Some(now);
        self.oled.show_banner(oled_text::OFF_TIME, self.banner_x);
        self.banner_x -= 3;
        if self.banner_x < -100 {
            self.banner_x = 128;
        }
    }
}
