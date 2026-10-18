//! HTTPS-клієнт Telegram Bot API поверх `esp_http_client`.

use std::time::Duration;

use esp_idf_svc::http::client::{Configuration, EspHttpConnection};
use esp_idf_svc::http::Method;
use esp_idf_svc::sys;
use serde::de::DeserializeOwned;
use serde::Serialize;
use shower_core::telegram::{
    parse_response, AnswerCallbackQuery, ApiError, DeleteMessage, EditMessageText, Message, PinChatMessage,
    SendMessage, Update,
};

const MAX_RESPONSE: usize = 32 * 1024;
const POLL_LIMIT: u32 = 20;

#[derive(Debug)]
pub enum TgError {
    /// Telegram відповів `ok: false`.
    Api(ApiError),
    /// Мережа, TLS, HTTP 5xx.
    Transport(anyhow::Error),
}

impl std::fmt::Display for TgError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TgError::Api(e) => write!(f, "API: {e}"),
            TgError::Transport(e) => write!(f, "{e:#}"),
        }
    }
}

impl From<serde_json::Error> for TgError {
    fn from(e: serde_json::Error) -> Self {
        TgError::Transport(e.into())
    }
}

pub type TgResult<T> = Result<T, TgError>;

/// Одне keep-alive з'єднання; кожен потік бота тримає свій клієнт.
pub struct Telegram {
    token: &'static str,
    timeout: Duration,
    conn: Option<EspHttpConnection>,
}

impl Telegram {
    pub fn new(token: &'static str, timeout: Duration) -> Self {
        Self { token, timeout, conn: None }
    }

    pub fn get_updates(&mut self, offset: i64, poll_timeout_s: u32) -> TgResult<Vec<Update>> {
        let body = format!(
            r#"{{"offset":{offset},"timeout":{poll_timeout_s},"limit":{POLL_LIMIT},"allowed_updates":["message","callback_query"]}}"#
        );
        self.call_raw("getUpdates", &body)
    }

    pub fn send(&mut self, msg: &SendMessage) -> TgResult<Message> {
        self.call("sendMessage", msg)
    }

    pub fn edit(&mut self, msg: &EditMessageText) -> TgResult<()> {
        self.call::<serde_json::Value>("editMessageText", msg).map(drop)
    }

    pub fn answer(&mut self, req: &AnswerCallbackQuery) -> TgResult<()> {
        self.call::<bool>("answerCallbackQuery", req).map(drop)
    }

    pub fn pin(&mut self, req: &PinChatMessage) -> TgResult<()> {
        self.call::<bool>("pinChatMessage", req).map(drop)
    }

    pub fn delete(&mut self, req: &DeleteMessage) -> TgResult<()> {
        self.call::<bool>("deleteMessage", req).map(drop)
    }

    fn call<R: DeserializeOwned>(&mut self, method: &str, req: &impl Serialize) -> TgResult<R> {
        let body = serde_json::to_string(req)?;
        self.call_raw(method, &body)
    }

    fn call_raw<R: DeserializeOwned>(&mut self, method: &str, body: &str) -> TgResult<R> {
        let resp = match self.request(method, body) {
            Ok(r) => r,
            Err(e) => {
                // Після збою з'єднання перестворюється з нуля.
                self.conn = None;
                return Err(TgError::Transport(e.context(method.to_owned())));
            }
        };
        parse_response(&resp).map_err(TgError::Api)
    }

    fn request(&mut self, method: &str, body: &str) -> anyhow::Result<Vec<u8>> {
        let conn = match &mut self.conn {
            Some(c) => c,
            slot @ None => slot.insert(EspHttpConnection::new(&Configuration {
                timeout: Some(self.timeout),
                crt_bundle_attach: Some(sys::esp_crt_bundle_attach),
                keep_alive_enable: true,
                ..Default::default()
            })?),
        };

        let url = format!("https://api.telegram.org/bot{}/{method}", self.token);
        let len = body.len().to_string();
        let headers = [("Content-Type", "application/json"), ("Content-Length", len.as_str())];
        conn.initiate_request(Method::Post, &url, &headers)?;
        conn.write_all(body.as_bytes())?;
        conn.initiate_response()?;
        let status = conn.status();

        let mut resp = Vec::new();
        let mut buf = [0u8; 512];
        loop {
            let n = conn.read(&mut buf)?;
            if n == 0 {
                break;
            }
            anyhow::ensure!(resp.len() + n <= MAX_RESPONSE, "відповідь більша за {MAX_RESPONSE} байт");
            resp.extend_from_slice(&buf[..n]);
        }
        // Помилки API (4xx) приходять із JSON-описом, їх розбирає parse_response.
        anyhow::ensure!(status < 500, "HTTP {status}");
        Ok(resp)
    }
}
