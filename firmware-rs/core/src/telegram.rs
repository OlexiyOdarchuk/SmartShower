//! Мінімальна частина Telegram Bot API, яка потрібна боту черги.

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
struct ApiResponse<T> {
    ok: bool,
    result: Option<T>,
    description: Option<String>,
}

/// Помилка, яку повернув сам Telegram (`ok: false`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ApiError {
    pub description: String,
}

impl ApiError {
    /// Текст не змінився — для редагування це не помилка.
    pub fn is_not_modified(&self) -> bool {
        self.description.contains("message is not modified")
    }

    /// Повідомлення вже не можна редагувати: видалене або недоступне.
    pub fn is_message_gone(&self) -> bool {
        ["message to edit not found", "message can't be edited", "MESSAGE_ID_INVALID"]
            .iter()
            .any(|s| self.description.contains(s))
    }
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.description)
    }
}

impl std::error::Error for ApiError {}

pub fn parse_response<T: DeserializeOwned>(body: &[u8]) -> Result<T, ApiError> {
    let resp: ApiResponse<T> =
        serde_json::from_slice(body).map_err(|e| ApiError { description: format!("невалідний JSON: {e}") })?;
    match (resp.ok, resp.result) {
        (true, Some(r)) => Ok(r),
        _ => Err(ApiError { description: resp.description.unwrap_or_else(|| "ok=false".into()) }),
    }
}

#[derive(Debug, Deserialize)]
pub struct Update {
    pub update_id: i64,
    pub message: Option<Message>,
    pub callback_query: Option<CallbackQuery>,
}

#[derive(Debug, Deserialize)]
pub struct Message {
    pub message_id: i64,
    pub from: Option<User>,
    pub chat: Chat,
    pub text: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CallbackQuery {
    pub id: String,
    pub from: User,
    pub message: Option<Message>,
    pub data: Option<String>,
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

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct InlineKeyboard {
    pub inline_keyboard: Vec<Vec<Button>>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Button {
    pub text: String,
    pub callback_data: String,
}

impl InlineKeyboard {
    pub fn row(buttons: &[(&str, CallbackAction)]) -> Self {
        let row = buttons
            .iter()
            .map(|(text, action)| Button { text: (*text).into(), callback_data: action.encode() })
            .collect();
        Self { inline_keyboard: vec![row] }
    }
}

#[derive(Debug, Serialize)]
pub struct SendMessage<'a> {
    pub chat_id: i64,
    pub text: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parse_mode: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reply_parameters: Option<ReplyParameters>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reply_markup: Option<&'a InlineKeyboard>,
}

#[derive(Debug, Serialize)]
pub struct ReplyParameters {
    pub message_id: i64,
}

impl<'a> SendMessage<'a> {
    pub fn text(chat_id: i64, text: &'a str) -> Self {
        Self { chat_id, text, parse_mode: None, reply_parameters: None, reply_markup: None }
    }

    pub fn html(chat_id: i64, text: &'a str) -> Self {
        Self { parse_mode: Some("HTML"), ..Self::text(chat_id, text) }
    }

    pub fn reply_to(mut self, message_id: i64) -> Self {
        self.reply_parameters = Some(ReplyParameters { message_id });
        self
    }

    pub fn keyboard(mut self, keyboard: &'a InlineKeyboard) -> Self {
        self.reply_markup = Some(keyboard);
        self
    }
}

/// HTML-текст; без `reply_markup` Telegram прибирає кнопки з повідомлення.
#[derive(Debug, Serialize)]
pub struct EditMessageText<'a> {
    pub chat_id: i64,
    pub message_id: i64,
    pub text: &'a str,
    pub parse_mode: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reply_markup: Option<&'a InlineKeyboard>,
}

impl<'a> EditMessageText<'a> {
    pub fn html(chat_id: i64, message_id: i64, text: &'a str) -> Self {
        Self { chat_id, message_id, text, parse_mode: "HTML", reply_markup: None }
    }
}

#[derive(Debug, Serialize)]
pub struct AnswerCallbackQuery<'a> {
    pub callback_query_id: &'a str,
    pub text: &'a str,
}

#[derive(Debug, Serialize)]
pub struct PinChatMessage {
    pub chat_id: i64,
    pub message_id: i64,
    pub disable_notification: bool,
}

#[derive(Debug, Serialize)]
pub struct DeleteMessage {
    pub chat_id: i64,
    pub message_id: i64,
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
    Status,
}

impl Command {
    const ALL: [(&'static str, Command); 8] = [
        ("/start", Command::Start),
        ("/get_info", Command::GetInfo),
        ("/queue", Command::Queue),
        ("/position", Command::Position),
        ("/join_to_queue", Command::Join),
        ("/leave_from_queue", Command::Leave),
        ("/clear_queue", Command::Clear),
        ("/status", Command::Status),
    ];

    /// Приймає `/cmd`, `/cmd@BotName` і `/cmd аргументи`.
    pub fn parse(text: &str) -> Option<Command> {
        Self::ALL.iter().find_map(|&(name, cmd)| {
            let rest = text.strip_prefix(name)?;
            (rest.is_empty() || rest.starts_with([' ', '@'])).then_some(cmd)
        })
    }
}

/// Дія інлайн-кнопки; кодується в `callback_data` (до 64 байт).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CallbackAction {
    Join,
    Leave,
    Position,
    /// Підтвердження виклику; id — кого кликали.
    Go(i64),
    /// Пропустити вперед наступного; id — кого кликали.
    Skip(i64),
}

impl CallbackAction {
    pub fn encode(&self) -> String {
        match self {
            CallbackAction::Join => "join".into(),
            CallbackAction::Leave => "leave".into(),
            CallbackAction::Position => "pos".into(),
            CallbackAction::Go(id) => format!("go:{id}"),
            CallbackAction::Skip(id) => format!("skip:{id}"),
        }
    }

    pub fn parse(data: &str) -> Option<Self> {
        match data.split_once(':') {
            None => match data {
                "join" => Some(CallbackAction::Join),
                "leave" => Some(CallbackAction::Leave),
                "pos" => Some(CallbackAction::Position),
                _ => None,
            },
            Some(("go", id)) => id.parse().ok().map(CallbackAction::Go),
            Some(("skip", id)) => id.parse().ok().map(CallbackAction::Skip),
            Some(_) => None,
        }
    }
}

pub fn html_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 8);
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
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
        assert_eq!(Command::parse("/status"), Some(Command::Status));
        assert_eq!(Command::parse("/queue_x"), None);
        assert_eq!(Command::parse("hello /start"), None);
    }

    #[test]
    fn callback_roundtrip() {
        for a in [
            CallbackAction::Join,
            CallbackAction::Leave,
            CallbackAction::Position,
            CallbackAction::Go(-100161234841),
            CallbackAction::Skip(7),
        ] {
            assert_eq!(CallbackAction::parse(&a.encode()), Some(a));
        }
        assert_eq!(CallbackAction::parse("go:abc"), None);
        assert_eq!(CallbackAction::parse("nope"), None);
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
        assert_eq!(html_escape("a<b>&\"_*"), "a&lt;b&gt;&amp;&quot;_*");
    }

    #[test]
    fn parses_updates_with_callbacks() {
        let body = br#"{"ok":true,"result":[
            {"update_id":10,"message":{"message_id":5,"from":{"id":7,"is_bot":false,"first_name":"Olha"},
             "chat":{"id":-100161234841,"type":"supergroup"},"date":0,"text":"/queue"}},
            {"update_id":11,"callback_query":{"id":"abc","from":{"id":7,"is_bot":false,"first_name":"Olha"},
             "message":{"message_id":9,"chat":{"id":-1,"type":"supergroup"},"date":0},
             "chat_instance":"x","data":"go:7"}},
            {"update_id":12,"edited_message":{}}
        ]}"#;
        let ups: Vec<Update> = parse_response(body).unwrap();
        assert_eq!(ups.len(), 3);
        assert_eq!(ups[0].message.as_ref().unwrap().chat.id, -100161234841);
        let cb = ups[1].callback_query.as_ref().unwrap();
        assert_eq!(cb.data.as_deref().and_then(CallbackAction::parse), Some(CallbackAction::Go(7)));
        assert_eq!(cb.message.as_ref().unwrap().message_id, 9);
        assert!(ups[2].message.is_none() && ups[2].callback_query.is_none());
    }

    #[test]
    fn reports_api_errors() {
        let body = br#"{"ok":false,"error_code":400,"description":"Bad Request: message is not modified: x"}"#;
        let err = parse_response::<Message>(body).unwrap_err();
        assert!(err.is_not_modified());
        assert!(!err.is_message_gone());
        let err = ApiError { description: "Bad Request: message to edit not found".into() };
        assert!(err.is_message_gone());
    }

    #[test]
    fn request_json() {
        let kb = InlineKeyboard::row(&[("Так", CallbackAction::Go(1))]);
        let json = serde_json::to_string(&SendMessage::text(-1, "hi").reply_to(5).keyboard(&kb)).unwrap();
        assert_eq!(
            json,
            r#"{"chat_id":-1,"text":"hi","reply_parameters":{"message_id":5},"reply_markup":{"inline_keyboard":[[{"text":"Так","callback_data":"go:1"}]]}}"#
        );
        let json = serde_json::to_string(&EditMessageText::html(3, 4, "<b>x</b>")).unwrap();
        assert_eq!(json, r#"{"chat_id":3,"message_id":4,"text":"<b>x</b>","parse_mode":"HTML"}"#);
    }
}
