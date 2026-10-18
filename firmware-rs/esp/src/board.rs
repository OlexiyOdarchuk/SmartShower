//! Розкладка пінів ESP32-WROOM (`upesy_wroom`), як у C++-прошивці.

use esp_idf_svc::hal::gpio::{AnyIOPin, AnyInputPin, AnyOutputPin, Pins};

pub struct BoardPins {
    pub shower_button: [AnyInputPin<'static>; 2],
    pub red_led: [AnyOutputPin<'static>; 2],
    pub green_led: [AnyOutputPin<'static>; 2],
    /// Спільний CLK обох TM1637.
    pub timer_clk: AnyOutputPin<'static>,
    pub timer_dio: [AnyIOPin<'static>; 2],
    pub queue_button: AnyInputPin<'static>,
    pub buzzer: AnyOutputPin<'static>,
    pub oled_sda: AnyIOPin<'static>,
    pub oled_scl: AnyIOPin<'static>,
    /// Кнопки температури 1..=4 для кожного душу. `None` — кнопка не опитується.
    pub temp_buttons: [[Option<AnyInputPin<'static>>; 4]; 2],
}

pub fn take(p: Pins) -> BoardPins {
    BoardPins {
        shower_button: [p.gpio23.into(), p.gpio4.into()],
        red_led: [p.gpio19.into(), p.gpio18.into()],
        green_led: [p.gpio17.into(), p.gpio5.into()],
        timer_clk: p.gpio14.into(),
        timer_dio: [p.gpio26.into(), p.gpio27.into()],
        queue_button: p.gpio16.into(),
        buzzer: p.gpio25.into(),
        oled_sda: p.gpio21.into(),
        oled_scl: p.gpio22.into(),
        // Вимкнено через залізо:
        //  - GPIO2 (S1 T2): onboard LED тягне до GND;
        //  - GPIO35/34 (S2 T1/T2): input-only без внутрішньої підтяжки, потрібен зовнішній pull-up 10k.
        temp_buttons: [
            [Some(p.gpio15.into()), None, Some(p.gpio13.into()), Some(p.gpio12.into())],
            [None, None, Some(p.gpio33.into()), Some(p.gpio32.into())],
        ],
    }
}
