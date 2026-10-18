//! Логіка SmartShower без залежностей від заліза.
//!
//! Усе, що тут лежить, тестується на ПК через `cargo test`; прошивка
//! (`firmware-rs/esp`) лише під'єднує ці типи до GPIO, NVS і HTTP.

pub mod input;
pub mod queue;
pub mod schedule;
pub mod segments;
pub mod shower;
pub mod stats;
pub mod telegram;
pub mod texts;
