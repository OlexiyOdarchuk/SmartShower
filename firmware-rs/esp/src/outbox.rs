//! Потік відправки: усі вихідні повідомлення й закріплений статус.
//!
//! Окремий від long-poll, щоб виклики й відповіді на кнопки йшли одразу,
//! а не після завершення чергового getUpdates.

use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::sync::Arc;
use std::time::Duration;

use log::{info, warn};
use shower_core::telegram::{
    AnswerCallbackQuery, DeleteMessage, EditMessageText, InlineKeyboard, PinChatMessage, SendMessage,
};
use shower_core::texts;

use crate::clock::now_ms;
use crate::secrets::{BOT_TOKEN, GROUP_ID};
use crate::state::Shared;
use crate::telegram::{Telegram, TgError};

const HTTP_TIMEOUT: Duration = Duration::from_secs(15);
const POLL_INTERVAL: Duration = Duration::from_millis(300);
const SEND_ATTEMPTS: u32 = 3;
const RETRY_DELAY: Duration = Duration::from_secs(2);
/// Не частіше за це редагуємо статус (ліміти Telegram на редагування в групі).
const STATUS_MIN_INTERVAL_MS: u64 = 4_000;
/// Навіть без змін оновлюємо статус, щоб рухалось очікування.
const STATUS_MAX_INTERVAL_MS: u64 = 60_000;

pub enum Job {
    /// Текстова відповідь у чат; `reply_to` — на яке повідомлення.
    Reply { chat_id: i64, reply_to: Option<i64>, text: String, html: bool },
    /// HTML-повідомлення в групу.
    Notice(String),
    /// Виклик користувача з кнопками «Йду» / «Пропустити вперед».
    Call { user_id: i64, name: String },
    /// Спливаюча відповідь на натискання кнопки.
    Answer { callback_id: String, text: String },
    /// Замінити текст повідомлення-виклику й прибрати кнопки.
    CloseCall { message_id: i64, text: String },
    /// Надіслати статус заново (і закріпити), старий видалити.
    RepostStatus,
}

pub struct Outbox {
    shared: Arc<Shared>,
    tg: Telegram,
    status_id: Option<i64>,
    last_status: Option<String>,
    status_at: Option<u64>,
    keyboard: InlineKeyboard,
}

impl Outbox {
    pub fn new(shared: Arc<Shared>) -> Self {
        let status_id = shared.status_message_id();
        Self {
            shared,
            tg: Telegram::new(BOT_TOKEN, HTTP_TIMEOUT),
            status_id,
            last_status: None,
            status_at: None,
            keyboard: texts::status_keyboard(),
        }
    }

    pub fn run(mut self, jobs: Receiver<Job>) -> ! {
        info!("Outbox: старт");
        let mut dirty = true;
        loop {
            match jobs.recv_timeout(POLL_INTERVAL) {
                Ok(job) => self.run_job(job),
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => std::thread::sleep(POLL_INTERVAL),
            }
            dirty |= self.shared.take_dirty();
            let now = now_ms();
            let since = self.status_at.map(|t| now - t);
            let due = match since {
                None => true,
                Some(s) => (dirty && s >= STATUS_MIN_INTERVAL_MS) || s >= STATUS_MAX_INTERVAL_MS,
            };
            if due {
                self.status_at = Some(now);
                dirty = !self.refresh_status();
            }
        }
    }

    fn run_job(&mut self, job: Job) {
        match job {
            Job::Reply { chat_id, reply_to, text, html } => {
                let mut msg = if html { SendMessage::html(chat_id, &text) } else { SendMessage::text(chat_id, &text) };
                if let Some(id) = reply_to {
                    msg = msg.reply_to(id);
                }
                self.send_with_retry(&msg);
            }
            Job::Notice(text) => {
                self.send_with_retry(&SendMessage::html(GROUP_ID, &text));
            }
            Job::Call { user_id, name } => {
                let text = texts::call(user_id, &name);
                let kb = texts::call_keyboard(user_id);
                self.send_with_retry(&SendMessage::html(GROUP_ID, &text).keyboard(&kb));
            }
            Job::Answer { callback_id, text } => {
                let req = AnswerCallbackQuery { callback_query_id: &callback_id, text: &text };
                if let Err(e) = self.tg.answer(&req) {
                    warn!("Outbox: answerCallbackQuery: {e}");
                }
            }
            Job::CloseCall { message_id, text } => {
                match self.tg.edit(&EditMessageText::html(GROUP_ID, message_id, &text)) {
                    Ok(()) => {}
                    Err(TgError::Api(e)) if e.is_not_modified() || e.is_message_gone() => {}
                    Err(e) => warn!("Outbox: editMessageText: {e}"),
                }
            }
            Job::RepostStatus => {
                if let Some(old) = self.status_id.take() {
                    let _ = self.tg.delete(&DeleteMessage { chat_id: GROUP_ID, message_id: old });
                }
                self.last_status = None;
                self.refresh_status();
            }
        }
    }

    fn send_with_retry(&mut self, msg: &SendMessage) {
        for attempt in 1..=SEND_ATTEMPTS {
            match self.tg.send(msg) {
                Ok(_) => return,
                // Помилку API повтор не виправить.
                Err(e @ TgError::Api(_)) => {
                    warn!("Outbox: sendMessage: {e}");
                    return;
                }
                Err(e) => {
                    warn!("Outbox: sendMessage (спроба {attempt}): {e}");
                    std::thread::sleep(RETRY_DELAY);
                }
            }
        }
    }

    /// Редагує закріплений статус або публікує новий. `true` — статус актуальний.
    fn refresh_status(&mut self) -> bool {
        let text = self.shared.status_text();
        if self.last_status.as_ref() == Some(&text) {
            return true;
        }
        if let Some(id) = self.status_id {
            let edit = EditMessageText { reply_markup: Some(&self.keyboard), ..EditMessageText::html(GROUP_ID, id, &text) };
            match self.tg.edit(&edit) {
                Ok(()) => {}
                Err(TgError::Api(e)) if e.is_not_modified() => {}
                Err(TgError::Api(e)) if e.is_message_gone() => {
                    info!("Outbox: статус-повідомлення зникло, публікую нове");
                    self.status_id = None;
                    return self.post_status(text);
                }
                Err(e) => {
                    warn!("Outbox: оновлення статусу: {e}");
                    return false;
                }
            }
            self.last_status = Some(text);
            return true;
        }
        self.post_status(text)
    }

    fn post_status(&mut self, text: String) -> bool {
        let msg = match self.tg.send(&SendMessage::html(GROUP_ID, &text).keyboard(&self.keyboard)) {
            Ok(m) => m,
            Err(e) => {
                warn!("Outbox: публікація статусу: {e}");
                return false;
            }
        };
        self.status_id = Some(msg.message_id);
        self.shared.save_status_message_id(msg.message_id);
        // Закріпити може лише адмін з правом закріплення; без нього статус просто лишається в чаті.
        let pin = PinChatMessage { chat_id: GROUP_ID, message_id: msg.message_id, disable_notification: true };
        if let Err(e) = self.tg.pin(&pin) {
            warn!("Outbox: закріпити статус не вдалося: {e}");
        }
        self.last_status = Some(text);
        true
    }
}
