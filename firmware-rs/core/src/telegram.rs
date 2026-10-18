//! Мінімальна частина Telegram Bot API, яка потрібна боту черги.

use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct ApiResponse<T> {
    pub ok: bool,
    pub result: Option<T>,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct Update {
    pub update_id: i64,
    pub message: Option<Message>,
}

#[derive(Debug, Deserialize)]
pub struct Message {
    pub message_id: i64,
    pub from: Option<User>,
    pub chat: Chat,
    pub text: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct User {
    pub id: i64,
    #[serde(default)]
    pub first_name: String,
    pub username: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct Chat {
    pub id: i64,
}

impl User {
    /// `@username`, інакше ім'я, інакше `User <id>`.
    pub fn display_name(&self) -> String {
        match &self.username {
            Some(u) if !u.is_empty() => format!("@{u}"),
            _ if !self.first_name.is_empty() => self.first_name.clone(),
            _ => format!("User {}", self.id),
        }
    }
}

pub fn parse_updates(body: &[u8]) -> Result<Vec<Update>, String> {
    let resp: ApiResponse<Vec<Update>> = serde_json::from_slice(body).map_err(|e| e.to_string())?;
    if !resp.ok {
        return Err(resp.description.unwrap_or_else(|| "ok=false".into()));
    }
    Ok(resp.result.unwrap_or_default())
}

#[derive(Debug, Serialize)]
pub struct SendMessage<'a> {
    pub chat_id: i64,
    pub text: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parse_mode: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reply_parameters: Option<ReplyParameters>,
}

#[derive(Debug, Serialize)]
pub struct ReplyParameters {
    pub message_id: i64,
}

impl<'a> SendMessage<'a> {
    pub fn text(chat_id: i64, text: &'a str) -> Self {
        Self { chat_id, text, parse_mode: None, reply_parameters: None }
    }

    pub fn html(chat_id: i64, text: &'a str) -> Self {
        Self { parse_mode: Some("HTML"), ..Self::text(chat_id, text) }
    }

    pub fn reply_to(mut self, message_id: i64) -> Self {
        self.reply_parameters = Some(ReplyParameters { message_id });
        self
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string(self).expect("SendMessage завжди серіалізується")
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Command {
    Start,
    GetInfo,
    Queue,
    Position,
    Join,
    Leave,
    Clear,
}

impl Command {
    const ALL: [(&'static str, Command); 7] = [
        ("/start", Command::Start),
        ("/get_info", Command::GetInfo),
        ("/queue", Command::Queue),
        ("/position", Command::Position),
        ("/join_to_queue", Command::Join),
        ("/leave_from_queue", Command::Leave),
        ("/clear_queue", Command::Clear),
    ];

    /// Приймає `/cmd`, `/cmd@BotName` і `/cmd аргументи`.
    pub fn parse(text: &str) -> Option<Command> {
        Self::ALL.iter().find_map(|&(name, cmd)| {
            let rest = text.strip_prefix(name)?;
            (rest.is_empty() || rest.starts_with([' ', '@'])).then_some(cmd)
        })
    }
}

pub fn html_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 8);
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            _ => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_parsing() {
        assert_eq!(Command::parse("/queue"), Some(Command::Queue));
        assert_eq!(Command::parse("/queue@ShowerBot"), Some(Command::Queue));
        assert_eq!(Command::parse("/join_to_queue please"), Some(Command::Join));
        assert_eq!(Command::parse("/queue_x"), None);
        assert_eq!(Command::parse("hello /start"), None);
        assert_eq!(Command::parse(""), None);
    }

    #[test]
    fn user_display_name_fallbacks() {
        let u = |username: Option<&str>, first: &str| User {
            id: 42,
            first_name: first.into(),
            username: username.map(Into::into),
        };
        assert_eq!(u(Some("vova"), "Вова").display_name(), "@vova");
        assert_eq!(u(None, "Вова").display_name(), "Вова");
        assert_eq!(u(Some(""), "").display_name(), "User 42");
    }

    #[test]
    fn escapes_html() {
        assert_eq!(html_escape("a<b>&_*"), "a&lt;b&gt;&amp;_*");
    }

    #[test]
    fn parses_get_updates() {
        let body = br#"{"ok":true,"result":[
            {"update_id":10,"message":{"message_id":5,"from":{"id":7,"is_bot":false,"first_name":"Olha"},
             "chat":{"id":-100161234841,"type":"supergroup"},"date":0,"text":"/queue"}},
            {"update_id":11,"edited_message":{}}
        ]}"#;
        let ups = parse_updates(body).unwrap();
        assert_eq!(ups.len(), 2);
        let m = ups[0].message.as_ref().unwrap();
        assert_eq!(m.chat.id, -100161234841);
        assert_eq!(m.from.as_ref().unwrap().display_name(), "Olha");
        assert!(ups[1].message.is_none());
    }

    #[test]
    fn reports_api_errors() {
        let body = br#"{"ok":false,"error_code":401,"description":"Unauthorized"}"#;
        assert_eq!(parse_updates(body).unwrap_err(), "Unauthorized");
    }

    #[test]
    fn send_message_json() {
        let json = SendMessage::text(-1, "hi").reply_to(5).to_json();
        assert_eq!(json, r#"{"chat_id":-1,"text":"hi","reply_parameters":{"message_id":5}}"#);
        let json = SendMessage::html(3, "<b>x</b>").to_json();
        assert_eq!(json, r#"{"chat_id":3,"text":"<b>x</b>","parse_mode":"HTML"}"#);
    }
}
