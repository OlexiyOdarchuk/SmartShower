//! Одна кабінка: фіксаційна кнопка на дверях, пара LED і таймер на TM1637.

use esp_idf_svc::hal::gpio::{AnyInputPin, AnyOutputPin, Input, Output, PinDriver, Pull};
use esp_idf_svc::sys::EspError;
use log::info;
use shower_core::segments;
use shower_core::shower::Occupant;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Transition {
    Occupied,
    Freed,
}

pub struct Shower {
    /// 1-based, для логів і текстів.
    pub number: usize,
    button: PinDriver<'static, Input>,
    red_led: PinDriver<'static, Output>,
    green_led: PinDriver<'static, Output>,
    busy: bool,
    timer_start_ms: u64,
    /// Що зараз на TM1637, щоб не смикати bit-bang без потреби.
    pub shown: Option<[u8; 4]>,
    pub occupant: Option<Occupant>,
}

impl Shower {
    pub fn new(
        number: usize,
        button: AnyInputPin<'static>,
        red_led: AnyOutputPin<'static>,
        green_led: AnyOutputPin<'static>,
    ) -> Result<Self, EspError> {
        let button = PinDriver::input(button, Pull::Up)?;
        let busy = button.is_low();
        let mut s = Self {
            number,
            button,
            red_led: PinDriver::output(red_led)?,
            green_led: PinDriver::output(green_led)?,
            busy,
            timer_start_ms: 0,
            shown: None,
            occupant: None,
        };
        s.apply_leds()?;
        info!("Shower {number}: init OK, busy={busy}");
        Ok(s)
    }

    pub fn is_busy(&self) -> bool {
        self.busy
    }

    /// Кнопка фіксаційна: LOW — душ зайнятий.
    pub fn poll(&mut self, now_ms: u64) -> Option<Transition> {
        let busy = self.button.is_low();
        if busy == self.busy {
            return None;
        }
        self.busy = busy;
        info!("Shower {}: -> {}", self.number, if busy { "BUSY" } else { "FREE" });
        let _ = self.apply_leds();
        if busy {
            self.timer_start_ms = now_ms;
            Some(Transition::Occupied)
        } else {
            Some(Transition::Freed)
        }
    }

    pub fn segments(&self, now_ms: u64) -> [u8; 4] {
        if self.busy {
            segments::timer(now_ms - self.timer_start_ms)
        } else {
            segments::FREE
        }
    }

    fn apply_leds(&mut self) -> Result<(), EspError> {
        if self.busy {
            self.red_led.set_high()?;
            self.green_led.set_low()
        } else {
            self.red_led.set_low()?;
            self.green_led.set_high()
        }
    }
}
