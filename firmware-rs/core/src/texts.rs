//! Тексти для бота й OLED.

use crate::queue::{JoinResult, Member, Queue};
use crate::shower::{water_info, WaterTemperature};
use crate::telegram::html_escape;

pub const START: &str = "👋 Привіт! Це бот для управління чергою в душ Гуртожитку №1.\n\n\
Доступні команди:\n\
/get_info — загальна інформація\n\
/queue — переглянути всю чергу\n\
/position — твоя позиція в черзі\n\
/join_to_queue — додатися в чергу\n\
/leave_from_queue — вийти з черги\n\
/clear_queue — очистити чергу (тільки адмін)";

pub const OFF_HOURS: &str = "❌ Зараз не робочий час.";
pub const LEFT: &str = "✅ Ви успішно вийшли з черги.";
pub const NOT_IN_QUEUE: &str = "❌ Вас немає в черзі.";
pub const ADMIN_ONLY: &str = "🚫 Ця команда доступна тільки адміну.";
pub const CLEARED: &str = "🧹 Чергу очищено.";

pub fn join(result: JoinResult, name: &str) -> String {
    match result {
        JoinResult::Added(pos) => format!("✅ {name}, ви додані в чергу! Ваша позиція: {pos}"),
        JoinResult::AlreadyIn(pos) => format!("⚠️ Ви вже в черзі! Ваша позиція: {pos}"),
        JoinResult::Full => "❌ Черга заповнена, спробуйте пізніше.".into(),
    }
}

pub fn queue_list(queue: &Queue) -> String {
    if queue.is_empty() {
        return "📭 Черга порожня.".into();
    }
    let mut out = format!("📋 Черга ({}):\n", queue.len());
    for (i, m) in queue.iter().enumerate() {
        out.push_str(&format!("{}. {}\n", i + 1, m.display_name()));
    }
    out
}

/// `index` — 0-based позиція, якщо користувач у черзі.
pub fn position(index: Option<usize>, len: usize) -> String {
    match index {
        None => "ℹ️ Вас немає в черзі. /join_to_queue щоб додатися.".into(),
        Some(i) => format!("📍 Ваша позиція: {} із {}", i + 1, len),
    }
}

pub fn info_report(showers: [Option<&WaterTemperature>; 2], queue_len: usize) -> String {
    format!(
        "📊 Загальна інформація:\n\n🚿 Душ 1:\n{}\n\n🚿 Душ 2:\n{}\n\nЧерга: {}",
        water_info(showers[0]),
        water_info(showers[1]),
        queue_len
    )
}

/// HTML-повідомлення зі згадкою. HTML замість MarkdownV2, бо username може
/// містити `_` і `*`, які зламали б розмітку.
pub fn notify_next(user_id: i64, name: &str) -> String {
    format!("🔔 Твоя черга наступна! <a href=\"tg://user?id={user_id}\">{}</a>", html_escape(name))
}

pub mod oled {
    use super::*;

    pub const OFF_TIME: &str = "OFF TIME";
    pub const NOT_WORKING: &str = "Не робочий час";
    pub const QUEUE_FULL: &str = "Черга повна";
    pub const WAIT: &str = "Зачекайте...";

    pub fn queue_status(len: usize, head: Option<&Member>) -> [String; 2] {
        let next = match head {
            None => "Empty".into(),
            Some(m) => format!("Next: {}", m.display_name()),
        };
        [format!("Черга: {len}"), next]
    }

    pub fn error(message: &str) -> [String; 2] {
        ["!".into(), message.into()]
    }

    /// `shower` і `level` 1-based.
    pub fn temperature_set(shower: usize, level: u8) -> String {
        format!("S{shower} T={level}")
    }

    pub fn no_occupant(shower: usize) -> String {
        format!("S{shower}: немає юзера")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shower::Occupant;

    #[test]
    fn queue_list_text() {
        let mut q = Queue::new();
        assert_eq!(queue_list(&q), "📭 Черга порожня.");
        q.try_join(1, "@a");
        q.push_button();
        assert_eq!(queue_list(&q), "📋 Черга (2):\n1. @a\n2. Кнопка\n");
    }

    #[test]
    fn join_texts() {
        assert_eq!(join(JoinResult::Added(3), "@a"), "✅ @a, ви додані в чергу! Ваша позиція: 3");
        assert_eq!(join(JoinResult::AlreadyIn(1), "@a"), "⚠️ Ви вже в черзі! Ваша позиція: 1");
    }

    #[test]
    fn position_text() {
        assert_eq!(position(Some(0), 4), "📍 Ваша позиція: 1 із 4");
        assert!(position(None, 4).starts_with("ℹ️"));
    }

    #[test]
    fn info_report_text() {
        let w = WaterTemperature { level: 4, occupant: Occupant::Button, time: "07:00:00".into() };
        let r = info_report([None, Some(&w)], 2);
        assert!(r.starts_with("📊 Загальна інформація:\n\n🚿 Душ 1:\nТемпература не встановлена\n\n🚿 Душ 2:\nТемпература: 4"));
        assert!(r.ends_with("\n\nЧерга: 2"));
    }

    #[test]
    fn notify_escapes_name() {
        assert_eq!(
            notify_next(7, "<b>"),
            "🔔 Твоя черга наступна! <a href=\"tg://user?id=7\">&lt;b&gt;</a>"
        );
    }

    #[test]
    fn oled_status() {
        assert_eq!(oled::queue_status(0, None), ["Черга: 0", "Empty"]);
        let m = Member::user(1, "@a");
        assert_eq!(oled::queue_status(1, Some(&m)), ["Черга: 1", "Next: @a"]);
    }
}
