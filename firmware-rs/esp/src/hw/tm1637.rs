//! Bit-bang драйвер двох TM1637 зі спільним CLK.
//!
//! Спільний CLK безпечний: поки DIO «чужого» дисплея відпущено (HIGH),
//! він не бачить умови START і ігнорує такти.

use esp_idf_svc::hal::delay::Ets;
use esp_idf_svc::hal::gpio::{AnyIOPin, AnyOutputPin, InputOutput, Output, PinDriver, Pull};
use esp_idf_svc::sys::EspError;

const CMD_DATA_AUTO_INC: u8 = 0x40;
const CMD_ADDR_0: u8 = 0xC0;
const CMD_DISPLAY_ON: u8 = 0x88;
const BIT_DELAY_US: u32 = 5;

pub struct Tm1637Pair {
    clk: PinDriver<'static, Output>,
    dio: [PinDriver<'static, InputOutput>; 2],
    brightness: u8,
}

impl Tm1637Pair {
    /// `brightness` 0..=7.
    pub fn new(clk: AnyOutputPin<'static>, dio: [AnyIOPin<'static>; 2], brightness: u8) -> Result<Self, EspError> {
        let mut clk = PinDriver::output(clk)?;
        clk.set_high()?;
        let [d0, d1] = dio;
        let mut dio = [PinDriver::input_output_od(d0, Pull::Up)?, PinDriver::input_output_od(d1, Pull::Up)?];
        for d in &mut dio {
            d.set_high()?;
        }
        Ok(Self { clk, dio, brightness: brightness.min(7) })
    }

    pub fn write(&mut self, display: usize, segments: [u8; 4]) -> Result<(), EspError> {
        self.start(display)?;
        self.byte(display, CMD_DATA_AUTO_INC)?;
        self.stop(display)?;

        self.start(display)?;
        self.byte(display, CMD_ADDR_0)?;
        for s in segments {
            self.byte(display, s)?;
        }
        self.stop(display)?;

        self.start(display)?;
        self.byte(display, CMD_DISPLAY_ON | self.brightness)?;
        self.stop(display)
    }

    fn start(&mut self, d: usize) -> Result<(), EspError> {
        self.dio[d].set_high()?;
        self.clk.set_high()?;
        Ets::delay_us(BIT_DELAY_US);
        self.dio[d].set_low()?;
        Ets::delay_us(BIT_DELAY_US);
        self.clk.set_low()
    }

    fn stop(&mut self, d: usize) -> Result<(), EspError> {
        self.clk.set_low()?;
        self.dio[d].set_low()?;
        Ets::delay_us(BIT_DELAY_US);
        self.clk.set_high()?;
        Ets::delay_us(BIT_DELAY_US);
        self.dio[d].set_high()?;
        Ets::delay_us(BIT_DELAY_US);
        Ok(())
    }

    /// Байт молодшим бітом уперед; ACK від чипа не перевіряємо.
    fn byte(&mut self, d: usize, mut b: u8) -> Result<(), EspError> {
        for _ in 0..8 {
            self.clk.set_low()?;
            if b & 1 != 0 {
                self.dio[d].set_high()?;
            } else {
                self.dio[d].set_low()?;
            }
            Ets::delay_us(BIT_DELAY_US);
            self.clk.set_high()?;
            Ets::delay_us(BIT_DELAY_US);
            b >>= 1;
        }
        // Такт ACK: відпускаємо DIO, чип тягне його вниз.
        self.clk.set_low()?;
        self.dio[d].set_high()?;
        Ets::delay_us(BIT_DELAY_US);
        self.clk.set_high()?;
        Ets::delay_us(BIT_DELAY_US);
        self.clk.set_low()
    }
}
