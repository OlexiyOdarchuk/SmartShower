//! SmartShower: черга в душові гуртожитку на ESP32 + Telegram-бот.
//!
//! Цикл заліза (`App`) крутиться в головній задачі; бот живе в окремому
//! потоці на ядрі 0, щоб блокуючі HTTP-запити не гальмували кнопки.
//! Спільний стан — `Arc<Shared>`.

mod app;
mod board;
mod bot;
mod config;
mod hw;
mod net;
mod secrets;
mod shower;
mod state;
mod telegram;

use std::sync::Arc;
use std::time::{Duration, Instant};

use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::hal::cpu::Core;
use esp_idf_svc::hal::delay::FreeRtos;
use esp_idf_svc::hal::peripherals::Peripherals;
use esp_idf_svc::hal::task::thread::ThreadSpawnConfiguration;
use esp_idf_svc::hal::task::watchdog::{TWDTConfig, TWDTDriver};
use esp_idf_svc::log::EspLogger;
use esp_idf_svc::nvs::EspDefaultNvsPartition;
use esp_idf_svc::wifi::{BlockingWifi, EspWifi};
use log::info;

use crate::app::App;
use crate::bot::Bot;
use crate::state::Shared;

const WDT_TIMEOUT: Duration = Duration::from_secs(15);
const BOT_STACK_SIZE: usize = 16 * 1024;

fn main() -> anyhow::Result<()> {
    esp_idf_svc::sys::link_patches();
    EspLogger::initialize_default();
    info!("--- SmartShower ---");

    let p = Peripherals::take()?;
    let sysloop = EspSystemEventLoop::take()?;
    let nvs = EspDefaultNvsPartition::take()?;

    let shared = Arc::new(Shared::new(nvs.clone())?);
    let pins = board::take(p.pins);
    let mut app = App::new(shared.clone(), pins, p.i2c0, p.ledc.timer0, p.ledc.channel0)?;

    let mut wifi = BlockingWifi::wrap(EspWifi::new(p.modem, sysloop.clone(), Some(nvs))?, sysloop)?;
    net::connect(&mut wifi)?;
    let _sntp = net::start_time_sync()?;

    // Бот на ядрі 0 (PRO_CPU); на одноядерному C3 це просто єдине ядро.
    ThreadSpawnConfiguration {
        name: Some(c"bot"),
        stack_size: BOT_STACK_SIZE,
        pin_to_core: Some(Core::Core0),
        ..Default::default()
    }
    .set()?;
    std::thread::Builder::new()
        .stack_size(BOT_STACK_SIZE)
        .spawn(move || Bot::new(shared).run(wifi))?;
    ThreadSpawnConfiguration::default().set()?;

    // Watchdog на цикл заліза: якщо щось зависне, плата перезавантажиться.
    let mut twdt = TWDTDriver::new(
        p.twdt,
        &TWDTConfig { duration: WDT_TIMEOUT, panic_on_trigger: true, ..Default::default() },
    )?;
    let mut watchdog = twdt.watch_current_task()?;
    info!("Task WDT: {WDT_TIMEOUT:?}");

    let boot = Instant::now();
    loop {
        watchdog.feed()?;
        app.run(boot.elapsed().as_millis() as u64);
        FreeRtos::delay_ms(1);
    }
}
