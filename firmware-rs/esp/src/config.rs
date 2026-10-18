use shower_core::schedule::{HourRange, Schedule};

/// Години включно з обох боків.
/// Ніч 00:00–05:59: черга закрита й очищається на початку ночі.
/// Перерва 11:00–13:59: черга заморожена, ставати в неї можна.
pub const SCHEDULE: Schedule = Schedule {
    night: HourRange::new(0, 5),
    breaks: &[HourRange::new(11, 13)],
};

/// Європа/Київ з автоматичним DST: EET = UTC+2 (зима), EEST = UTC+3 (літо).
/// Літній час з останньої неділі березня 03:00 до останньої неділі жовтня 04:00.
pub const TZ: &str = "EET-2EEST,M3.5.0/3,M10.5.0/4";
pub const NTP_SERVERS: [&str; 3] = ["0.ua.pool.ntp.org", "1.ua.pool.ntp.org", "pool.ntp.org"];
