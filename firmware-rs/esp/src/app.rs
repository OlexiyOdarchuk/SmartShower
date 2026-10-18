//! Цикл заліза: кнопки, таймери, табло, buzzer, авто-вибуття.

use std::sync::Arc;

use esp_idf_svc::hal::gpio::{Input, PinDriver, Pull};
use esp_idf_svc::hal::i2c::{I2cConfig, I2cDriver, I2C0};
use esp_idf_svc::hal::ledc::{CHANNEL0, TIMER0};
use esp_idf_svc::hal::units::Hertz;
use log::{info, warn};
use shower_core::autoremove::AutoRemove;
use shower_core::input::{DebouncedButton, Edge, RateLimit};
use shower_core::queue::Queue;
use shower_core::shower::{Occupant, WaterTemperature};
use shower_core::texts::oled as oled_text;

use crate::board::BoardPins;
use crate::config::SCHEDULE;
use crate::hw::buzzer::Buzzer;
use crate::hw::oled::Oled;
use crate::hw::tm1637::Tm1637Pair;
use crate::net;
use crate::shower::{Shower, Transition};
use crate::state::Shared;

const TEMP_DEBOUNCE_MS: u64 = 30;
const TEMP_REFIRE_MS: u64 = 300;
const QUEUE_BTN_REFIRE_MS: u64 = 2000;
const ERROR_DISPLAY_MS: u64 = 2000;
const OLED_REFRESH_MS: u64 = 500;
const ANIM_FRAME_MS: u64 = 100;
const WORKING_TIME_TTL_MS: u64 = 1000;
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
    auto_remove: AutoRemove,
    working: bool,
    working_checked_at: Option<u64>,
    error: Option<(String, u64)>,
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
            auto_remove: AutoRemove::default(),
            working: true,
            working_checked_at: None,
            error: None,
            status_shown_at: None,
            banner_x: 0,
            banner_frame_at: None,
        })
    }

    pub fn run(&mut self, now: u64) {
        self.refresh_working_time(now);
        if let Some(is_error) = self.shared.take_beep() {
            self.buzzer.beep(is_error, now);
        }
        self.buzzer.tick(now);

        // Темп-кнопки і звільнення душів обробляємо незалежно від часу доби.
        self.poll_temp_buttons(now);
        let transitions = [self.showers[0].poll(now), self.showers[1].poll(now)];
        for (shower, t) in self.showers.iter_mut().zip(transitions) {
            if t == Some(Transition::Freed) {
                shower.occupant = None;
            }
        }

        if !self.working {
            self.shared.modify(Queue::clear);
            self.animate_off_time(now);
            self.update_timers(now);
            return;
        }

        for (i, t) in transitions.into_iter().enumerate() {
            if t == Some(Transition::Occupied) {
                self.on_occupied(i, now);
            }
        }
        self.poll_queue_button(now);
        self.run_auto_remove(now);
        self.update_timers(now);
        self.update_oled(now);
    }

    fn refresh_working_time(&mut self, now: u64) {
        if self.working_checked_at.is_some_and(|t| now - t < WORKING_TIME_TTL_MS) {
            return;
        }
        self.working = SCHEDULE.is_working(net::local_hour());
        self.working_checked_at = Some(now);
        self.shared.set_working_time(self.working);
    }

    fn show_error(&mut self, message: impl Into<String>, now: u64) {
        self.error = Some((message.into(), now));
    }

    /// Хтось зайшов у кабінку: забираємо першого з черги, або позначаємо як «кнопкою».
    fn on_occupied(&mut self, shower: usize, now: u64) {
        let taken = self.shared.modify(Queue::pop_head);
        self.showers[shower].occupant = Some(taken.as_ref().map_or(Occupant::Button, Occupant::from));
        self.buzzer.beep(false, now);
        if taken.is_some() {
            // Голова черги змінилась — сповістити нового першого.
            self.shared.request_notify_next();
        }
    }

    fn poll_queue_button(&mut self, now: u64) {
        if !self.queue_edge.rising(self.queue_button.is_low()) {
            return;
        }
        if !self.queue_rate.try_fire(now) {
            self.buzzer.beep(true, now);
            self.show_error(oled_text::WAIT, now);
            return;
        }
        if self.shared.modify(Queue::push_button) {
            self.buzzer.beep(false, now);
        } else {
            self.buzzer.beep(true, now);
            self.show_error(oled_text::QUEUE_FULL, now);
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
            self.buzzer.beep(true, now);
            self.show_error(oled_text::no_occupant(shower + 1), now);
            return;
        };
        let water = WaterTemperature { level, occupant, time: net::local_time_string() };
        info!("Shower {}: T={level} {:?} о {}", shower + 1, water.occupant, water.time);
        self.shared.set_water(shower, water);
        self.buzzer.beep(false, now);
        self.show_error(oled_text::temperature_set(shower + 1, level), now);
    }

    fn run_auto_remove(&mut self, now: u64) {
        let head = self.shared.head();
        let any_free = self.showers.iter().any(|s| !s.is_busy());
        if let Some(id) = self.auto_remove.tick(head.as_ref(), any_free, now) {
            info!("Queue: авто-вибуття {id} після таймауту");
            self.shared.modify(|q| q.remove_head_if(id));
            self.shared.request_notify_next();
        }
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
        if let Some((message, since)) = &self.error {
            if now - since < ERROR_DISPLAY_MS {
                self.oled.show_lines(oled_text::error(message));
                return;
            }
            self.error = None;
        }
        if self.status_shown_at.is_some_and(|t| now - t <= OLED_REFRESH_MS) {
            return;
        }
        let lines = self.shared.read(|q| oled_text::queue_status(q.len(), q.head()));
        self.oled.show_lines(lines);
        self.status_shown_at = Some(now);
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
