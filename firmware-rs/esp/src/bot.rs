//! Потік прийому: long-polling Telegram, команди й натискання кнопок.
//! Усі відповіді йдуть через [`Job`] у потік відправки.

use std::sync::Arc;
use std::time::Duration;

use log::{info, warn};
use shower_core::queue::{JoinResult, SkipResult};
use shower_core::telegram::{CallbackAction, CallbackQuery, Command, Message, User};
use shower_core::texts;

use crate::clock::now_ms;
use crate::net::{self, Wifi};
use crate::outbox::Job;
use crate::secrets::{ADMIN_ID, BOT_TOKEN, GROUP_ID};
use crate::state::Shared;
use crate::telegram::Telegram;

const POLL_TIMEOUT_S: u32 = 25;
const HTTP_TIMEOUT: Duration = Duration::from_secs(POLL_TIMEOUT_S as u64 + 10);
const RETRY_DELAY: Duration = Duration::from_secs(3);

pub struct Bot {
    shared: Arc<Shared>,
    tg: Telegram,
    offset: i64,
}

impl Bot {
    pub fn new(shared: Arc<Shared>) -> Self {
        Self { shared, tg: Telegram::new(BOT_TOKEN, HTTP_TIMEOUT), offset: 0 }
    }

    pub fn run(mut self, mut wifi: Wifi) -> ! {
        info!("Bot: старт");
        loop {
            if !net::ensure_connected(&mut wifi) {
                std::thread::sleep(RETRY_DELAY);
                continue;
            }
            match self.tg.get_updates(self.offset, POLL_TIMEOUT_S) {
                Ok(updates) => {
                    for u in updates {
                        self.offset = u.update_id + 1;
                        if let Some(msg) = u.message {
                            self.on_message(&msg);
                        }
                        if let Some(cb) = u.callback_query {
                            self.on_callback(&cb);
                        }
                    }
                }
                Err(e) => {
                    warn!("Bot: getUpdates: {e}");
                    std::thread::sleep(RETRY_DELAY);
                }
            }
        }
    }

    fn reply(&self, to: &Message, text: String) {
        self.shared.send(Job::Reply { chat_id: to.chat.id, reply_to: Some(to.message_id), text, html: false });
    }

    /// Відповідь із пиком buzzer'а: `ok = false` — помилковий тон.
    fn reply_beep(&self, to: &Message, text: String, ok: bool) {
        self.shared.request_beep(!ok);
        self.reply(to, text);
    }

    fn on_message(&self, msg: &Message) {
        if msg.chat.id != GROUP_ID {
            return;
        }
        let Some(cmd) = msg.text.as_deref().and_then(Command::parse) else {
            return;
        };
        let shared = &self.shared;

        match (cmd, &msg.from) {
            (Command::Start, _) => {
                shared.send(Job::Reply { chat_id: msg.chat.id, reply_to: None, text: texts::START.into(), html: false });
            }
            (Command::GetInfo, _) => self.reply(msg, shared.info_report()),
            (Command::Queue, _) => shared.send(Job::Reply {
                chat_id: msg.chat.id,
                reply_to: Some(msg.message_id),
                text: shared.status_text(),
                html: true,
            }),
            (Command::Status, _) => shared.send(Job::RepostStatus),
            // Решта команд прив'язана до користувача; без відправника (пост від імені чату) — ігноруємо.
            (_, None) => {}
            (Command::Position, Some(from)) => self.reply(msg, self.position_text(from)),
            (Command::Join, Some(from)) => {
                let (text, ok) = self.join(from);
                self.reply_beep(msg, text, ok);
            }
            (Command::Leave, Some(from)) => {
                let (text, ok) = self.leave(from);
                self.reply_beep(msg, text, ok);
            }
            (Command::Clear, Some(from)) if from.id != ADMIN_ID => {
                self.reply_beep(msg, texts::ADMIN_ONLY.into(), false);
            }
            (Command::Clear, Some(_)) => {
                shared.modify(|q| q.clear());
                self.reply_beep(msg, texts::CLEARED.into(), true);
            }
        }
    }

    fn on_callback(&self, cb: &CallbackQuery) {
        if cb.message.as_ref().is_some_and(|m| m.chat.id != GROUP_ID) {
            return;
        }
        let Some(action) = cb.data.as_deref().and_then(CallbackAction::parse) else {
            return self.answer(cb, String::new());
        };
        let from = &cb.from;
        let text = match action {
            CallbackAction::Join => {
                let (text, ok) = self.join(from);
                self.shared.request_beep(!ok);
                text
            }
            CallbackAction::Leave => {
                let (text, ok) = self.leave(from);
                self.shared.request_beep(!ok);
                text
            }
            CallbackAction::Position => self.position_text(from),
            CallbackAction::Go(id) | CallbackAction::Skip(id) if id != from.id => texts::NOT_YOUR_CALL.into(),
            CallbackAction::Go(id) => {
                if self.shared.modify(|q| q.confirm(id, now_ms())) {
                    self.close_call(cb, texts::call_confirmed(&from.display_name()));
                    texts::GOING.into()
                } else {
                    self.close_stale_call(cb);
                    texts::CALL_STALE.into()
                }
            }
            CallbackAction::Skip(id) => match self.shared.modify(|q| q.skip(id)) {
                SkipResult::Moved(pos) => {
                    self.close_call(cb, texts::call_skipped(&from.display_name(), pos));
                    texts::skipped_toast(pos)
                }
                SkipResult::NobodyBehind => texts::NOBODY_BEHIND.into(),
                SkipResult::NotInQueue => {
                    self.close_stale_call(cb);
                    texts::CALL_STALE.into()
                }
            },
        };
        self.answer(cb, text);
    }

    fn answer(&self, cb: &CallbackQuery, text: String) {
        self.shared.send(Job::Answer { callback_id: cb.id.clone(), text });
    }

    fn close_call(&self, cb: &CallbackQuery, text: String) {
        if let Some(m) = &cb.message {
            self.shared.send(Job::CloseCall { message_id: m.message_id, text });
        }
    }

    fn close_stale_call(&self, cb: &CallbackQuery) {
        self.close_call(cb, format!("⌛ {}", texts::CALL_STALE));
    }

    /// Текст відповіді і чи це успіх (для тону buzzer'а).
    fn join(&self, from: &User) -> (String, bool) {
        let name = from.display_name();
        match self.shared.try_join(from.id, &name) {
            Err(_) => (texts::CLOSED.into(), false),
            Ok((r, eta)) => (texts::join(r, &name, eta), matches!(r, JoinResult::Added(_))),
        }
    }

    fn leave(&self, from: &User) -> (String, bool) {
        match self.shared.modify(|q| q.leave(from.id)) {
            Some(_) => (texts::LEFT.into(), true),
            None => (texts::NOT_IN_QUEUE.into(), false),
        }
    }

    fn position_text(&self, from: &User) -> String {
        let (index, len, eta) = self.shared.position(from.id);
        texts::position(index, len, eta)
    }
}
