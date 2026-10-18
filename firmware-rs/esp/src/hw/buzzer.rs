//! Неблокуючий buzzer на LEDC: [`Buzzer::beep`] вмикає тон, [`Buzzer::tick`] вимикає по таймеру.

use esp_idf_svc::hal::gpio::AnyOutputPin;
use esp_idf_svc::hal::ledc::config::TimerConfig;
use esp_idf_svc::hal::ledc::{LedcDriver, LedcTimerDriver, Resolution, CHANNEL0, TIMER0};
use esp_idf_svc::hal::units::Hertz;
use esp_idf_svc::sys::{self, EspError};
use log::{info, warn};

pub struct Buzzer {
    channel: LedcDriver<'static>,
    off_at_ms: Option<u64>,
}

impl Buzzer {
    pub fn new(timer: TIMER0<'static>, channel: CHANNEL0<'static>, pin: AnyOutputPin<'static>) -> Result<Self, EspError> {
        let timer = LedcTimerDriver::new(timer, &TimerConfig::new().frequency(Hertz(1000)).resolution(Resolution::Bits10))?;
        let mut channel = LedcDriver::new(channel, timer, pin)?;
        channel.set_duty(0)?;
        Ok(Self { channel, off_at_ms: None })
    }

    pub fn beep(&mut self, is_error: bool, now_ms: u64) {
        info!("Buzzer: {}", if is_error { "ERROR" } else { "OK" });
        let (freq, dur_ms) = if is_error { (400, 200) } else { (1000, 80) };
        // LedcDriver володіє таймером, тож частоту міняємо напряму через ESP-IDF.
        // TIMER0 — low-speed таймер на всіх чипах.
        let res = sys::esp!(unsafe { sys::ledc_set_freq(sys::ledc_mode_t_LEDC_LOW_SPEED_MODE, sys::ledc_timer_t_LEDC_TIMER_0, freq) })
            .and_then(|()| self.channel.set_duty(self.channel.get_max_duty() / 2));
        match res {
            Ok(()) => self.off_at_ms = Some(now_ms + dur_ms),
            Err(e) => warn!("Buzzer: {e}"),
        }
    }

    pub fn tick(&mut self, now_ms: u64) {
        if self.off_at_ms.is_some_and(|t| now_ms >= t) {
            self.off_at_ms = None;
            let _ = self.channel.set_duty(0);
        }
    }
}
