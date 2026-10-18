//! Тексти для бота й OLED.

use crate::queue::{JoinResult, Queue, Status, CALL_TIMEOUT_MS, MAX_TIMEOUTS};
use crate::schedule::Mode;
use crate::shower::{water_info, WaterTemperature};
use crate::telegram::{html_escape, CallbackAction, InlineKeyboard};

pub const START: &str = "👋 Привіт! Це бот черги в душ Гуртожитку №1.\n\n\
Стан черги — у закріпленому повідомленні, там же кнопки «Стати в чергу» і «Вийти». \
Коли звільниться душ, бот покличе тебе: натисни «Йду» або «Пропустити вперед».\n\n\
Команди:\n\
/status — показати стан черги з кнопками\n\
/get_info — температура води в душах\n\
/queue — переглянути чергу\n\
/position — твоя позиція в черзі\n\
/join_to_queue — додатися в чергу\n\
/leave_from_queue — вийти з черги\n\
/clear_queue — очистити чергу (тільки адмін)";

pub const CLOSED: &str = "🌙 Зараз неробочий час, черга закрита.";
pub const LEFT: &str = "✅ Ви вийшли з черги.";
pub const NOT_IN_QUEUE: &str = "❌ Вас немає в черзі.";
pub const ADMIN_ONLY: &str = "🚫 Ця команда доступна тільки адміну.";
pub const CLEARED: &str = "🧹 Чергу очищено.";
pub const NOT_YOUR_CALL: &str = "Цей виклик для іншої людини.";
pub const CALL_STALE: &str = "Цей виклик уже неактуальний.";
pub const GOING: &str = "🚿 Душ чекає на вас!";
pub const NOBODY_BEHIND: &str = "Позаду нікого немає — можна йти.";

/// «≈ N хв» або «одразу».
pub fn eta(ms: u64) -> String {
    match ms.div_ceil(60_000) {
        0 => "одразу".into(),
        m => format!("≈ {m} хв"),
    }
}

fn hour(h: u8) -> String {
    format!("{h:02}:00")
}

pub fn join(result: JoinResult, name: &str, eta_ms: u64) -> String {
    match result {
        JoinResult::Added(pos) => format!("✅ {name}, ви в черзі! Позиція: {pos}, очікування: {}.", eta(eta_ms)),
        JoinResult::AlreadyIn(pos) => format!("⚠️ Ви вже в черзі. Позиція: {pos}, очікування: {}.", eta(eta_ms)),
        JoinResult::Full => "❌ Черга заповнена, спробуйте пізніше.".into(),
    }
}

/// `index` — 0-based позиція, якщо користувач у черзі.
pub fn position(index: Option<usize>, len: usize, eta_ms: u64) -> String {
    match index {
        None => "ℹ️ Вас немає в черзі.".into(),
        Some(i) => format!("📍 Ваша позиція: {} із {}, очікування: {}.", i + 1, len, eta(eta_ms)),
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

pub struct StatusView<'a> {
    pub mode: Mode,
    pub busy: [bool; 2],
    pub queue: &'a Queue,
    /// Очікування для кожної позиції черги, мс.
    pub etas: &'a [u64],
}

/// HTML-текст закріпленого статус-повідомлення.
pub fn status(v: &StatusView) -> String {
    let mut out = String::new();
    for (i, busy) in v.busy.iter().enumerate() {
        let state = if *busy { "🔴 зайнято" } else { "🟢 вільно" };
        out.push_str(&format!("<b>🚿 Душ {}:</b> {state}\n", i + 1));
    }
    out.push('\n');
    match v.mode {
        Mode::Open => {}
        Mode::Break { until_hour } => out.push_str(&format!(
            "⏸ Перерва до {}: викликів немає, ставати в чергу можна.\n\n",
            hour(until_hour)
        )),
        Mode::Night { until_hour } => {
            out.push_str(&format!("🌙 Неробочий час до {}, черга закрита.", hour(until_hour)));
            return out;
        }
    }
    if v.queue.is_empty() {
        out.push_str("📭 Черга порожня.");
        return out;
    }
    out.push_str(&format!("<b>📋 Черга ({}):</b>\n", v.queue.len()));
    for (i, e) in v.queue.iter().enumerate() {
        let note = match (e.status, v.mode) {
            (Status::Called { .. }, _) => "🔔 викликано".into(),
            (Status::Confirmed { .. }, _) => "✅ іде".into(),
            (Status::Waiting, Mode::Open) => eta(v.etas.get(i).copied().unwrap_or(0)),
            (Status::Waiting, _) => String::new(),
        };
        out.push_str(&format!("{}. {}", i + 1, html_escape(&e.member.display_name())));
        if !note.is_empty() {
            out.push_str(" — ");
            out.push_str(&note);
        }
        out.push('\n');
    }
    out
}

pub fn status_keyboard() -> InlineKeyboard {
    InlineKeyboard::row(&[
        ("➕ Стати в чергу", CallbackAction::Join),
        ("📍 Позиція", CallbackAction::Position),
        ("➖ Вийти", CallbackAction::Leave),
    ])
}

fn mention(user_id: i64, name: &str) -> String {
    format!("<a href=\"tg://user?id={user_id}\">{}</a>", html_escape(name))
}

/// Виклик зі згадкою. HTML замість MarkdownV2, бо username може містити `_` і `*`.
pub fn call(user_id: i64, name: &str) -> String {
    format!(
        "🔔 {}, ваша черга — душ вільний!\nПідтвердіть за {} хв, інакше черга піде далі.",
        mention(user_id, name),
        CALL_TIMEOUT_MS / 60_000
    )
}

pub fn call_keyboard(user_id: i64) -> InlineKeyboard {
    InlineKeyboard::row(&[("✅ Йду", CallbackAction::Go(user_id)), ("⏭ Пропустити вперед", CallbackAction::Skip(user_id))])
}

pub fn call_confirmed(name: &str) -> String {
    format!("✅ {} іде в душ.", html_escape(name))
}

pub fn call_skipped(name: &str, position: usize) -> String {
    format!("⏭ {} пропускає наступного вперед, тепер позиція {position}.", html_escape(name))
}

pub fn skipped_toast(position: usize) -> String {
    format!("⏭ Ваша нова позиція: {position}.")
}

pub fn timeout_moved(name: &str, position: usize) -> String {
    format!("⏰ Час на виклик для {} минув, тепер позиція {position}.", html_escape(name))
}

pub fn timeout_removed(name: &str) -> String {
    format!("❌ {} прибрано з черги після {MAX_TIMEOUTS} пропущених викликів.", html_escape(name))
}

pub mod oled {
    use super::*;

    pub const OFF_TIME: &str = "OFF TIME";
    pub const CLOSED: &str = "Не робочий час";
    pub const QUEUE_FULL: &str = "Черга повна";
    pub const WAIT: &str = "Зачекайте...";

    pub fn queue_status(queue: &Queue) -> [String; 2] {
        let called = queue.iter().find(|e| e.status.is_active());
        let line = match (called, queue.get(0)) {
            (Some(e), _) => format!("Заходь: {}", e.member.display_name()),
            (None, Some(e)) => format!("Далі: {}", e.member.display_name()),
            (None, None) => "Порожньо".into(),
        };
        [format!("Черга: {}", queue.len()), line]
    }

    pub fn break_status(until_hour: u8, queue_len: usize) -> [String; 2] {
        [format!("Перерва до {}", hour(until_hour)), format!("Черга: {queue_len}")]
    }

    pub fn ticket(n: u16) -> [String; 2] {
        ["Ваш талон:".into(), format!("№{n}")]
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
    fn eta_format() {
        assert_eq!(eta(0), "одразу");
        assert_eq!(eta(1), "≈ 1 хв");
        assert_eq!(eta(10 * 60_000), "≈ 10 хв");
    }

    #[test]
    fn join_texts() {
        assert_eq!(
            join(JoinResult::Added(3), "@a", 20 * 60_000),
            "✅ @a, ви в черзі! Позиція: 3, очікування: ≈ 20 хв."
        );
        assert!(join(JoinResult::AlreadyIn(1), "@a", 0).starts_with("⚠️ Ви вже в черзі. Позиція: 1"));
    }

    #[test]
    fn position_text() {
        assert_eq!(position(Some(0), 4, 0), "📍 Ваша позиція: 1 із 4, очікування: одразу.");
        assert_eq!(position(None, 4, 0), "ℹ️ Вас немає в черзі.");
    }

    #[test]
    fn info_report_text() {
        let w = WaterTemperature { level: 4, occupant: Occupant::WalkIn, time: "07:00:00".into() };
        let r = info_report([None, Some(&w)], 2);
        assert!(r.starts_with("📊 Загальна інформація:\n\n🚿 Душ 1:\nТемпература не встановлена\n\n🚿 Душ 2:\nТемпература: 4"));
        assert!(r.ends_with("\n\nЧерга: 2"));
    }

    fn sample_queue() -> Queue {
        let mut q = Queue::new();
        q.try_join(1, "@a<b>");
        q.take_ticket();
        q.try_join(3, "Olha");
        q.tick(1, 0);
        q
    }

    #[test]
    fn status_open() {
        let q = sample_queue();
        let s = status(&StatusView { mode: Mode::Open, busy: [true, false], queue: &q, etas: &[0, 0, 600_000] });
        assert_eq!(
            s,
            "<b>🚿 Душ 1:</b> 🔴 зайнято\n<b>🚿 Душ 2:</b> 🟢 вільно\n\n\
             <b>📋 Черга (3):</b>\n1. @a&lt;b&gt; — 🔔 викликано\n2. Талон №1 — одразу\n3. Olha — ≈ 10 хв\n"
        );
    }

    #[test]
    fn status_break_and_night() {
        let mut q = sample_queue();
        q.reset_calls();
        let s = status(&StatusView { mode: Mode::Break { until_hour: 14 }, busy: [false; 2], queue: &q, etas: &[] });
        assert!(s.contains("⏸ Перерва до 14:00"));
        assert!(s.contains("1. @a&lt;b&gt;\n"), "у перерву без ETA");
        let s = status(&StatusView { mode: Mode::Night { until_hour: 6 }, busy: [false; 2], queue: &q, etas: &[] });
        assert!(s.ends_with("🌙 Неробочий час до 06:00, черга закрита."));
        let empty = Queue::new();
        let s = status(&StatusView { mode: Mode::Open, busy: [false; 2], queue: &empty, etas: &[] });
        assert!(s.ends_with("📭 Черга порожня."));
    }

    #[test]
    fn call_text_escapes_name() {
        assert_eq!(
            call(7, "<b>"),
            "🔔 <a href=\"tg://user?id=7\">&lt;b&gt;</a>, ваша черга — душ вільний!\nПідтвердіть за 5 хв, інакше черга піде далі."
        );
        assert_eq!(call_keyboard(7).inline_keyboard[0][1].callback_data, "skip:7");
    }

    #[test]
    fn oled_status() {
        assert_eq!(oled::queue_status(&Queue::new()), ["Черга: 0", "Порожньо"]);
        let mut q = Queue::new();
        q.try_join(1, "@a");
        assert_eq!(oled::queue_status(&q), ["Черга: 1", "Далі: @a"]);
        q.tick(1, 0);
        assert_eq!(oled::queue_status(&q), ["Черга: 1", "Заходь: @a"]);
        assert_eq!(oled::ticket(7), ["Ваш талон:", "№7"]);
    }
}
