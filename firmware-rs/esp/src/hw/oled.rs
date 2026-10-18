//! Табло SSD1306 128×32.

use embedded_graphics::mono_font::{ascii, iso_8859_5, MonoTextStyle};
use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::prelude::*;
use embedded_graphics::text::{Baseline, Text};
use esp_idf_svc::hal::i2c::I2cDriver;
use log::warn;
use ssd1306::mode::BufferedGraphicsMode;
use ssd1306::prelude::*;
use ssd1306::{I2CDisplayInterface, Ssd1306};

type Display = Ssd1306<I2CInterface<I2cDriver<'static>>, DisplaySize128x32, BufferedGraphicsMode<DisplaySize128x32>>;

pub struct Oled {
    display: Display,
    last: Option<[String; 2]>,
}

impl Oled {
    pub fn new(i2c: I2cDriver<'static>) -> Self {
        let mut display = Ssd1306::new(I2CDisplayInterface::new(i2c), DisplaySize128x32, DisplayRotation::Rotate0)
            .into_buffered_graphics_mode();
        // Без табло черга працює далі, тому помилку лише логуємо.
        if let Err(e) = display.init() {
            warn!("OLED: init не вдався: {e:?}");
        }
        Self { display, last: None }
    }

    /// Два рядки; перемальовує, лише якщо текст змінився.
    pub fn show_lines(&mut self, lines: [String; 2]) {
        if self.last.as_ref() == Some(&lines) {
            return;
        }
        let style = MonoTextStyle::new(&iso_8859_5::FONT_6X10, BinaryColor::On);
        self.display.clear_buffer();
        for (line, y) in lines.iter().zip([0, 16]) {
            let _ = Text::with_baseline(line, Point::new(0, y), style, Baseline::Top).draw(&mut self.display);
        }
        let _ = self.display.flush();
        self.last = Some(lines);
    }

    /// Великий текст зі зсувом `x` для бігучого рядка.
    pub fn show_banner(&mut self, text: &str, x: i32) {
        let style = MonoTextStyle::new(&ascii::FONT_10X20, BinaryColor::On);
        self.display.clear_buffer();
        let _ = Text::with_baseline(text, Point::new(x, 6), style, Baseline::Top).draw(&mut self.display);
        let _ = self.display.flush();
        self.last = None;
    }
}
