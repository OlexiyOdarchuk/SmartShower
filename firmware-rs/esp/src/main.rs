//! SmartShower: черга в душові гуртожитку на ESP32-WROOM + Telegram-бот.
//!
//! Ядро 1: цикл заліза (`App`) — кнопки, таймери, табло, виклики черги.
//! Ядро 0: Wi-Fi і два потоки бота — прийом (long-poll) і відправка, кожен
//! зі своїм TLS-з'єднанням, щоб довгий getUpdates не затримував виклики.
//! Спільний стан — `Arc<Shared>`, завдання на відправку — канал `Job`.

mod app;
mod board;
mod bot;
mod clock;
mod config;
mod hw;
mod net;
mod outbox;
mod secrets;
mod shower;
mod state;
mod telegram;

use std::ffi::CStr;
use std::sync::{mpsc, Arc};
use std::time::Duration;

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
use crate::outbox::Outbox;
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

    let (jobs_tx, jobs_rx) = mpsc::channel();
    let shared = Arc::new(Shared::new(nvs.clone(), jobs_tx)?);
    let pins = board::take(p.pins);
    let mut app = App::new(shared.clone(), pins, p.i2c0, p.ledc.timer0, p.ledc.channel0)?;

    let mut wifi = BlockingWifi::wrap(EspWifi::new(p.modem, sysloop.clone(), Some(nvs))?, sysloop)?;
    net::connect(&mut wifi)?;
    let _sntp = net::start_time_sync()?;

    // HTTP-з'єднання не можна передавати між потоками, тож клієнти створюються вже в них.
    let bot_shared = shared.clone();
    spawn_on_core0(c"bot", move || Bot::new(bot_shared).run(wifi))?;
    spawn_on_core0(c"outbox", move || Outbox::new(shared).run(jobs_rx))?;

    // Watchdog на цикл заліза: якщо щось зависне, плата перезавантажиться.
    let mut twdt = TWDTDriver::new(
        p.twdt,
        &TWDTConfig { duration: WDT_TIMEOUT, panic_on_trigger: true, ..Default::default() },
    )?;
    let mut watchdog = twdt.watch_current_task()?;
    info!("Task WDT: {WDT_TIMEOUT:?}");

    loop {
        watchdog.feed()?;
        app.run(clock::now_ms());
        FreeRtos::delay_ms(1);
    }
}

/// Потік бота на ядрі 0 (PRO_CPU), подалі від циклу заліза.
fn spawn_on_core0(name: &'static CStr, f: impl FnOnce() + Send + 'static) -> anyhow::Result<()> {
    ThreadSpawnConfiguration {
        name: Some(name),
        stack_size: BOT_STACK_SIZE,
        pin_to_core: Some(Core::Core0),
        ..Default::default()
    }
    .set()?;
    std::thread::Builder::new().stack_size(BOT_STACK_SIZE).spawn(f)?;
    ThreadSpawnConfiguration::default().set()?;
    Ok(())
}
