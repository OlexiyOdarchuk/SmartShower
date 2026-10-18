//! Бот-потік: long-polling Telegram і відповіді на команди.

use std::sync::Arc;
use std::time::Duration;

use log::{info, warn};
use shower_core::queue::JoinResult;
use shower_core::telegram::{Command, Message, SendMessage};
use shower_core::texts;

use crate::net::{self, Wifi};
use crate::secrets::{ADMIN_ID, BOT_TOKEN, GROUP_ID};
use crate::state::Shared;
use crate::telegram::Telegram;

const RETRY_DELAY: Duration = Duration::from_secs(3);

pub struct Bot {
    shared: Arc<Shared>,
    tg: Telegram,
    offset: i64,
}

impl Bot {
    pub fn new(shared: Arc<Shared>) -> Self {
        Self { shared, tg: Telegram::new(BOT_TOKEN), offset: 0 }
    }

    pub fn run(mut self, mut wifi: Wifi) -> ! {
        info!("Bot: старт");
        loop {
            if !net::ensure_connected(&mut wifi) {
                std::thread::sleep(RETRY_DELAY);
                continue;
            }
            match self.tg.get_updates(self.offset) {
                Ok(updates) => {
                    for u in updates {
                        self.offset = u.update_id + 1;
                        if let Some(msg) = u.message {
                            self.handle(&msg);
                        }
                    }
                }
                Err(e) => {
                    warn!("Bot: getUpdates: {e:#}");
                    std::thread::sleep(RETRY_DELAY);
                }
            }
            if self.shared.take_notify_next() {
                self.notify_next();
            }
        }
    }

    fn send(&mut self, msg: SendMessage) {
        if let Err(e) = self.tg.send(&msg) {
            warn!("Bot: sendMessage: {e:#}");
        }
    }

    fn reply(&mut self, to: &Message, text: &str) {
        self.send(SendMessage::text(to.chat.id, text).reply_to(to.message_id));
    }

    /// Відповідь із пиком buzzer'а: `ok = false` — помилковий тон.
    fn reply_beep(&mut self, to: &Message, text: &str, ok: bool) {
        self.shared.request_beep(!ok);
        self.reply(to, text);
    }

    fn notify_next(&mut self) {
        let Some(head) = self.shared.head() else { return };
        let Some(id) = head.user_id() else { return };
        let text = texts::notify_next(id, &head.display_name());
        self.send(SendMessage::html(GROUP_ID, &text));
    }

    fn handle(&mut self, msg: &Message) {
        if msg.chat.id != GROUP_ID {
            return;
        }
        let Some(cmd) = msg.text.as_deref().and_then(Command::parse) else {
            return;
        };
        let shared = Arc::clone(&self.shared);

        match (cmd, &msg.from) {
            (Command::Start, _) => self.send(SendMessage::text(msg.chat.id, texts::START)),
            (Command::GetInfo, _) => self.reply(msg, &shared.info_report()),
            (Command::Queue, _) => self.reply(msg, &shared.read(texts::queue_list)),
            // Решта команд прив'язана до користувача; без відправника (пост від імені чату) — ігноруємо.
            (_, None) => {}
            (Command::Position, Some(from)) => {
                let text = shared.read(|q| texts::position(q.index_of(from.id), q.len()));
                self.reply(msg, &text);
            }
            (Command::Join, Some(from)) => {
                let name = from.display_name();
                match shared.try_join(from.id, &name) {
                    None => self.reply_beep(msg, texts::OFF_HOURS, false),
                    Some(r) => self.reply_beep(msg, &texts::join(r, &name), matches!(r, JoinResult::Added(_))),
                }
            }
            (Command::Leave, Some(from)) => match shared.modify(|q| q.leave(from.id)) {
                None => self.reply_beep(msg, texts::NOT_IN_QUEUE, false),
                Some(index) => {
                    self.reply_beep(msg, texts::LEFT, true);
                    if index == 0 {
                        self.notify_next();
                    }
                }
            },
            (Command::Clear, Some(from)) if from.id != ADMIN_ID => self.reply_beep(msg, texts::ADMIN_ONLY, false),
            (Command::Clear, Some(_)) => {
                shared.modify(|q| q.clear());
                self.reply_beep(msg, texts::CLEARED, true);
            }
        }
    }
}
