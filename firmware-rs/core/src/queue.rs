use std::collections::VecDeque;

pub const CAPACITY: usize = 30;

// Роздільники для серіалізації в NVS (Unit Separator / Record Separator,
// гарантовано не зустрічаються в username/first_name). Формат сумісний
// з C++-прошивкою, тож черга переживає перепрошивку.
const FIELD_SEP: char = '\x1F';
const RECORD_SEP: char = '\x1E';
const BUTTON_ID: &str = "0";

/// Хто стоїть у черзі.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Member {
    /// Анонім, що натиснув фізичну кнопку черги.
    Button,
    User { id: i64, name: String },
}

impl Member {
    pub fn user(id: i64, name: impl Into<String>) -> Self {
        Member::User { id, name: name.into() }
    }

    pub fn user_id(&self) -> Option<i64> {
        match self {
            Member::Button => None,
            Member::User { id, .. } => Some(*id),
        }
    }

    pub fn display_name(&self) -> String {
        match self {
            Member::Button => "Кнопка".into(),
            Member::User { name, .. } if !name.is_empty() => name.clone(),
            Member::User { id, .. } => id.to_string(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum JoinResult {
    /// Позиції 1-based.
    Added(usize),
    AlreadyIn(usize),
    Full,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Queue {
    entries: VecDeque<Member>,
}

impl Queue {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn is_full(&self) -> bool {
        self.entries.len() >= CAPACITY
    }

    pub fn head(&self) -> Option<&Member> {
        self.entries.front()
    }

    pub fn iter(&self) -> impl Iterator<Item = &Member> {
        self.entries.iter()
    }

    /// 0-based індекс користувача в черзі.
    pub fn index_of(&self, user_id: i64) -> Option<usize> {
        self.entries.iter().position(|m| m.user_id() == Some(user_id))
    }

    pub fn try_join(&mut self, user_id: i64, name: &str) -> JoinResult {
        if let Some(i) = self.index_of(user_id) {
            return JoinResult::AlreadyIn(i + 1);
        }
        if self.is_full() {
            return JoinResult::Full;
        }
        self.entries.push_back(Member::user(user_id, name));
        JoinResult::Added(self.entries.len())
    }

    /// Додає анонімного користувача з кнопки. `false`, якщо черга повна.
    pub fn push_button(&mut self) -> bool {
        if self.is_full() {
            return false;
        }
        self.entries.push_back(Member::Button);
        true
    }

    /// Прибирає користувача; повертає його 0-based індекс до видалення.
    pub fn leave(&mut self, user_id: i64) -> Option<usize> {
        let i = self.index_of(user_id)?;
        self.entries.remove(i);
        Some(i)
    }

    pub fn pop_head(&mut self) -> Option<Member> {
        self.entries.pop_front()
    }

    /// Прибирає голову, лише якщо це досі той самий користувач.
    pub fn remove_head_if(&mut self, user_id: i64) -> bool {
        if self.head().and_then(Member::user_id) == Some(user_id) {
            self.entries.pop_front();
            true
        } else {
            false
        }
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }

    pub fn serialize(&self) -> String {
        let mut out = String::with_capacity(self.entries.len() * 32);
        for (i, m) in self.entries.iter().enumerate() {
            if i > 0 {
                out.push(RECORD_SEP);
            }
            let (id, name) = match m {
                Member::Button => (BUTTON_ID.to_string(), ""),
                Member::User { id, name } => (id.to_string(), name.as_str()),
            };
            out.push_str(&id);
            out.push(FIELD_SEP);
            out.push_str(name);
        }
        out
    }

    /// Пошкоджені записи пропускає; зайве понад [`CAPACITY`] відкидає.
    pub fn deserialize(s: &str) -> Self {
        let mut q = Queue::new();
        for record in s.split(RECORD_SEP).filter(|r| !r.is_empty()) {
            let Some((id, name)) = record.split_once(FIELD_SEP) else {
                break;
            };
            if q.is_full() {
                break;
            }
            let member = if id == BUTTON_ID {
                Member::Button
            } else {
                match id.parse() {
                    Ok(id) => Member::user(id, name),
                    Err(_) => continue,
                }
            };
            q.entries.push_back(member);
        }
        q
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn join_reports_positions_and_duplicates() {
        let mut q = Queue::new();
        assert_eq!(q.try_join(1, "@a"), JoinResult::Added(1));
        assert_eq!(q.try_join(2, "@b"), JoinResult::Added(2));
        assert_eq!(q.try_join(1, "@a"), JoinResult::AlreadyIn(1));
        assert_eq!(q.len(), 2);
    }

    #[test]
    fn join_and_button_respect_capacity() {
        let mut q = Queue::new();
        for i in 0..CAPACITY as i64 {
            q.try_join(i + 1, "x");
        }
        assert_eq!(q.try_join(999, "y"), JoinResult::Full);
        assert!(!q.push_button());
    }

    #[test]
    fn leave_keeps_order() {
        let mut q = Queue::new();
        q.try_join(1, "a");
        q.push_button();
        q.try_join(2, "b");
        assert_eq!(q.leave(1), Some(0));
        assert_eq!(q.leave(1), None);
        assert_eq!(q.head(), Some(&Member::Button));
        assert_eq!(q.index_of(2), Some(1));
    }

    #[test]
    fn remove_head_if_only_matches_current_head() {
        let mut q = Queue::new();
        q.try_join(1, "a");
        q.try_join(2, "b");
        assert!(!q.remove_head_if(2));
        assert!(q.remove_head_if(1));
        assert_eq!(q.head().and_then(Member::user_id), Some(2));
    }

    #[test]
    fn display_names() {
        assert_eq!(Member::Button.display_name(), "Кнопка");
        assert_eq!(Member::user(5, "").display_name(), "5");
        assert_eq!(Member::user(5, "@vova").display_name(), "@vova");
    }

    #[test]
    fn serialization_roundtrip() {
        let mut q = Queue::new();
        q.try_join(1433760480, "@vova");
        q.push_button();
        q.try_join(-7, "Ім'я з пробілом");
        q.try_join(8, "");
        assert_eq!(Queue::deserialize(&q.serialize()), q);
        assert_eq!(Queue::deserialize(""), Queue::new());
    }

    #[test]
    fn reads_cpp_firmware_format() {
        let stored = "1433760480\x1F@vova\x1E0\x1F\x1E42\x1FOlha";
        let q = Queue::deserialize(stored);
        let names: Vec<_> = q.iter().map(Member::display_name).collect();
        assert_eq!(names, ["@vova", "Кнопка", "Olha"]);
    }

    #[test]
    fn skips_garbage_records() {
        let q = Queue::deserialize("abc\x1Fx\x1E5\x1Fok");
        assert_eq!(q.len(), 1);
        assert_eq!(q.head().and_then(Member::user_id), Some(5));
    }
}
