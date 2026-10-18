//! Wi-Fi, NTP і локальний час.

use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use anyhow::Context;
use esp_idf_svc::sntp::{EspSntp, SntpConf};
use esp_idf_svc::sys;
use esp_idf_svc::wifi::{AuthMethod, BlockingWifi, ClientConfiguration, Configuration, EspWifi};
use log::{info, warn};

use crate::config::{NTP_SERVERS, TZ};
use crate::secrets::{WIFI_PASS, WIFI_SSID};

pub type Wifi = BlockingWifi<EspWifi<'static>>;

const CONNECT_TIMEOUT: Duration = Duration::from_secs(30);

/// Підключається до Wi-Fi; якщо за 30 с не вдалося — перезавантажує плату.
pub fn connect(wifi: &mut Wifi) -> anyhow::Result<()> {
    wifi.set_configuration(&Configuration::Client(ClientConfiguration {
        ssid: WIFI_SSID.try_into().ok().context("WIFI_SSID довший за 32 байти")?,
        password: WIFI_PASS.try_into().ok().context("WIFI_PASS довший за 64 байти")?,
        auth_method: if WIFI_PASS.is_empty() { AuthMethod::None } else { AuthMethod::WPA2Personal },
        ..Default::default()
    }))?;
    wifi.start()?;

    let started = Instant::now();
    while let Err(e) = wifi.connect() {
        if started.elapsed() > CONNECT_TIMEOUT {
            warn!("WiFi: не вдалося підключитися за {CONNECT_TIMEOUT:?}, перезавантаження");
            esp_idf_svc::hal::reset::restart();
        }
        warn!("WiFi: {e}, повтор");
        std::thread::sleep(Duration::from_millis(500));
    }
    wifi.wait_netif_up()?;
    info!("WiFi: підключено, IP={}", wifi.wifi().sta_netif().get_ip_info()?.ip);
    Ok(())
}

/// Відновлює з'єднання, якщо воно впало. `false` — мережі поки немає.
pub fn ensure_connected(wifi: &mut Wifi) -> bool {
    if wifi.is_connected().unwrap_or(false) {
        return true;
    }
    warn!("WiFi: з'єднання втрачено, перепідключення");
    match wifi.connect().and_then(|()| wifi.wait_netif_up()) {
        Ok(()) => {
            info!("WiFi: відновлено");
            true
        }
        Err(e) => {
            warn!("WiFi: {e}");
            false
        }
    }
}

/// Запускає NTP і задає таймзону. Повернений `EspSntp` треба тримати живим.
pub fn start_time_sync() -> anyhow::Result<EspSntp<'static>> {
    std::env::set_var("TZ", TZ);
    unsafe { sys::tzset() };
    let mut conf = SntpConf::default();
    for (slot, server) in conf.servers.iter_mut().zip(NTP_SERVERS) {
        *slot = server;
    }
    Ok(EspSntp::new(&conf)?)
}

fn local_tm() -> Option<sys::tm> {
    // Раніше 2020-01-01 — значить NTP ще не синхронізувався.
    const SANE_EPOCH: u64 = 1_577_836_800;
    let secs = SystemTime::now().duration_since(UNIX_EPOCH).ok()?.as_secs();
    if secs < SANE_EPOCH {
        return None;
    }
    let t = secs as sys::time_t;
    let mut tm: sys::tm = unsafe { core::mem::zeroed() };
    unsafe { sys::localtime_r(&t, &mut tm) };
    Some(tm)
}

/// Локальна година, або `None`, поки час не синхронізовано.
pub fn local_hour() -> Option<u8> {
    local_tm().map(|tm| tm.tm_hour as u8)
}

/// «ГГ:ХХ:СС», або «--:--:--», поки час не синхронізовано.
pub fn local_time_string() -> String {
    match local_tm() {
        Some(tm) => format!("{:02}:{:02}:{:02}", tm.tm_hour, tm.tm_min, tm.tm_sec),
        None => "--:--:--".into(),
    }
}
