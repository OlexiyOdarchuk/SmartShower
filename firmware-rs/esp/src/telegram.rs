//! HTTPS-клієнт Telegram Bot API поверх `esp_http_client`.

use std::time::Duration;

use anyhow::{bail, Context};
use esp_idf_svc::http::client::{Configuration, EspHttpConnection};
use esp_idf_svc::http::Method;
use esp_idf_svc::sys;
use shower_core::telegram::{parse_updates, SendMessage, Update};

/// Long-poll тримаємо коротким: поки запит висить, бот не може надіслати
/// «твоя черга наступна», а друге TLS-з'єднання для C3 задороге по пам'яті.
const POLL_TIMEOUT_S: u32 = 5;
const HTTP_TIMEOUT: Duration = Duration::from_secs(POLL_TIMEOUT_S as u64 + 10);
const MAX_RESPONSE: usize = 32 * 1024;
const POLL_LIMIT: u32 = 20;

pub struct Telegram {
    token: &'static str,
    conn: Option<EspHttpConnection>,
}

impl Telegram {
    pub fn new(token: &'static str) -> Self {
        Self { token, conn: None }
    }

    pub fn get_updates(&mut self, offset: i64) -> anyhow::Result<Vec<Update>> {
        let body = format!(
            r#"{{"offset":{offset},"timeout":{POLL_TIMEOUT_S},"limit":{POLL_LIMIT},"allowed_updates":["message"]}}"#
        );
        let resp = self.call("getUpdates", &body)?;
        parse_updates(&resp).map_err(anyhow::Error::msg)
    }

    pub fn send(&mut self, msg: &SendMessage) -> anyhow::Result<()> {
        self.call("sendMessage", &msg.to_json()).map(drop)
    }

    /// Після будь-якої помилки з'єднання перестворюється з нуля.
    fn call(&mut self, method: &str, body: &str) -> anyhow::Result<Vec<u8>> {
        let result = self.call_inner(method, body);
        if result.is_err() {
            self.conn = None;
        }
        result
    }

    fn call_inner(&mut self, method: &str, body: &str) -> anyhow::Result<Vec<u8>> {
        let conn = match &mut self.conn {
            Some(c) => c,
            slot @ None => slot.insert(EspHttpConnection::new(&Configuration {
                timeout: Some(HTTP_TIMEOUT),
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
            if resp.len() + n > MAX_RESPONSE {
                bail!("{method}: відповідь більша за {MAX_RESPONSE} байт");
            }
            resp.extend_from_slice(&buf[..n]);
        }

        // getUpdates з ok=false пояснює помилку в тілі, тож 4xx віддаємо парсеру.
        if status >= 500 || (method != "getUpdates" && status != 200) {
            let text = String::from_utf8_lossy(&resp);
            return Err(anyhow::anyhow!("HTTP {status}: {text}")).context(method.to_owned());
        }
        Ok(resp)
    }
}
