//! Черга з викликами: перші в черзі отримують виклик, коли звільняється
//! кабінка, і мають підтвердити, що йдуть.

use std::collections::VecDeque;

pub const CAPACITY: usize = 30;
/// Скільки дається на підтвердження виклику і на дорогу до душу після нього.
pub const CALL_TIMEOUT_MS: u64 = 5 * 60 * 1000;
/// Після стількох прострочених викликів людина вибуває з черги.
pub const MAX_TIMEOUTS: u8 = 3;
const MAX_TICKET: u16 = 99;

// Роздільники для серіалізації в NVS (Unit Separator / Record Separator,
// гарантовано не зустрічаються в username/first_name).
const FIELD_SEP: char = '\x1F';
const RECORD_SEP: char = '\x1E';
const TICKET_PREFIX: char = 't';
/// Анонімний запис із C++-прошивки; при відновленні отримує новий талон.
const LEGACY_BUTTON_ID: &str = "0";

/// Хто стоїть у черзі.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Member {
    /// Талон з фізичної кнопки черги.
    Ticket(u16),
    User { id: i64, name: String },
}

impl Member {
    pub fn user(id: i64, name: impl Into<String>) -> Self {
        Member::User { id, name: name.into() }
    }

    pub fn user_id(&self) -> Option<i64> {
        match self {
            Member::Ticket(_) => None,
            Member::User { id, .. } => Some(*id),
        }
    }

    pub fn display_name(&self) -> String {
        match self {
            Member::Ticket(n) => format!("Талон №{n}"),
            Member::User { name, .. } if !name.is_empty() => name.clone(),
            Member::User { id, .. } => id.to_string(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    Waiting,
    /// Кабінка вільна, людину покликали.
    Called { since_ms: u64 },
    /// Людина підтвердила, що йде.
    Confirmed { since_ms: u64 },
}

impl Status {
    pub fn is_active(&self) -> bool {
        !matches!(self, Status::Waiting)
    }

    fn since_ms(&self) -> Option<u64> {
        match *self {
            Status::Waiting => None,
            Status::Called { since_ms } | Status::Confirmed { since_ms } => Some(since_ms),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    pub member: Member,
    pub status: Status,
    /// Скільки викликів людина вже прострочила.
    pub timeouts: u8,
}

impl Entry {
    fn new(member: Member) -> Self {
        Self { member, status: Status::Waiting, timeouts: 0 }
    }
}

/// Що сталося в черзі з ініціативи системи; про це варто сповістити.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Event {
    Called(Member),
    /// Виклик прострочено; людину зсунуто вниз (позиція 1-based).
    Moved { member: Member, position: usize },
    /// Прострочено [`MAX_TIMEOUTS`] викликів.
    Removed(Member),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum JoinResult {
    /// Позиції 1-based.
    Added(usize),
    AlreadyIn(usize),
    Full,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SkipResult {
    /// Нова позиція, 1-based.
    Moved(usize),
    /// Позаду немає нікого, хто чекає, пропускати нікого.
    NobodyBehind,
    NotInQueue,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Queue {
    entries: VecDeque<Entry>,
    last_ticket: u16,
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

    pub fn iter(&self) -> impl Iterator<Item = &Entry> {
        self.entries.iter()
    }

    /// 0-based індекс користувача в черзі.
    pub fn index_of(&self, user_id: i64) -> Option<usize> {
        self.entries.iter().position(|e| e.member.user_id() == Some(user_id))
    }

    pub fn get(&self, index: usize) -> Option<&Entry> {
        self.entries.get(index)
    }

    pub fn try_join(&mut self, user_id: i64, name: &str) -> JoinResult {
        if let Some(i) = self.index_of(user_id) {
            return JoinResult::AlreadyIn(i + 1);
        }
        if self.is_full() {
            return JoinResult::Full;
        }
        self.entries.push_back(Entry::new(Member::user(user_id, name)));
        JoinResult::Added(self.entries.len())
    }

    /// Видає наступний вільний номер талона (1..=99 по колу).
    pub fn take_ticket(&mut self) -> Option<u16> {
        if self.is_full() {
            return None;
        }
        let n = self.next_ticket_number();
        self.entries.push_back(Entry::new(Member::Ticket(n)));
        Some(n)
    }

    fn next_ticket_number(&mut self) -> u16 {
        let in_use = |n: u16| self.entries.iter().any(|e| e.member == Member::Ticket(n));
        let mut n = self.last_ticket;
        loop {
            n = n % MAX_TICKET + 1;
            if !in_use(n) {
                self.last_ticket = n;
                return n;
            }
        }
    }

    /// Прибирає користувача; повертає його 0-based індекс до видалення.
    pub fn leave(&mut self, user_id: i64) -> Option<usize> {
        let i = self.index_of(user_id)?;
        self.entries.remove(i);
        Some(i)
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// Скасовує всі виклики (наприклад, на час перерви).
    pub fn reset_calls(&mut self) {
        for e in &mut self.entries {
            e.status = Status::Waiting;
        }
    }

    /// `false` — користувача зараз не викликали.
    pub fn confirm(&mut self, user_id: i64, now_ms: u64) -> bool {
        let Some(e) = self.index_of(user_id).map(|i| &mut self.entries[i]) else {
            return false;
        };
        match e.status {
            Status::Waiting => false,
            Status::Called { .. } => {
                e.status = Status::Confirmed { since_ms: now_ms };
                true
            }
            Status::Confirmed { .. } => true,
        }
    }

    /// Пропустити вперед наступного, хто чекає.
    pub fn skip(&mut self, user_id: i64) -> SkipResult {
        let Some(i) = self.index_of(user_id) else {
            return SkipResult::NotInQueue;
        };
        if !self.has_waiting_after(i) {
            return SkipResult::NobodyBehind;
        }
        self.entries[i].status = Status::Waiting;
        match self.move_below_next_waiting(i) {
            Some(new) => SkipResult::Moved(new + 1),
            None => SkipResult::NobodyBehind,
        }
    }

    /// Хтось зайшов у кабінку. Вважаємо, що це перший, хто підтвердив виклик,
    /// інакше перший викликаний. `None` — зайшли поза чергою.
    pub fn on_enter(&mut self) -> Option<Member> {
        let i = self
            .entries
            .iter()
            .position(|e| matches!(e.status, Status::Confirmed { .. }))
            .or_else(|| self.entries.iter().position(|e| matches!(e.status, Status::Called { .. })))?;
        self.entries.remove(i).map(|e| e.member)
    }

    /// Прострочує виклики й кличе стільки людей, скільки вільних кабінок.
    pub fn tick(&mut self, free_showers: usize, now_ms: u64) -> Vec<Event> {
        let mut events = Vec::new();

        let mut i = 0;
        while i < self.entries.len() {
            let e = &mut self.entries[i];
            let expired = e.status.since_ms().is_some_and(|t| now_ms.saturating_sub(t) >= CALL_TIMEOUT_MS);
            if !expired {
                i += 1;
                continue;
            }
            e.timeouts += 1;
            e.status = Status::Waiting;
            if e.timeouts >= MAX_TIMEOUTS {
                let e = self.entries.remove(i).expect("індекс у межах");
                events.push(Event::Removed(e.member));
                continue;
            }
            let member = e.member.clone();
            match self.move_below_next_waiting(i) {
                // На місці i тепер інший запис, тому i не збільшуємо.
                Some(new) => events.push(Event::Moved { member, position: new + 1 }),
                // Позаду нікого — людину буде покликано знову нижче.
                None => i += 1,
            }
        }

        let mut active = self.entries.iter().filter(|e| e.status.is_active()).count();
        // Кабінок поменшало: знімаємо виклики з найнижчих, хто ще не підтвердив.
        for e in self.entries.iter_mut().rev() {
            if active <= free_showers {
                break;
            }
            if matches!(e.status, Status::Called { .. }) {
                e.status = Status::Waiting;
                active -= 1;
            }
        }
        for e in self.entries.iter_mut() {
            if active >= free_showers {
                break;
            }
            if e.status == Status::Waiting {
                e.status = Status::Called { since_ms: now_ms };
                active += 1;
                events.push(Event::Called(e.member.clone()));
            }
        }
        events
    }

    fn has_waiting_after(&self, i: usize) -> bool {
        self.entries.iter().skip(i + 1).any(|e| e.status == Status::Waiting)
    }

    /// Ставить запис `i` одразу за наступним, хто чекає. Повертає новий індекс.
    fn move_below_next_waiting(&mut self, i: usize) -> Option<usize> {
        let j = (i + 1..self.entries.len()).find(|&j| self.entries[j].status == Status::Waiting)?;
        let e = self.entries.remove(i).expect("індекс у межах");
        self.entries.insert(j, e);
        Some(j)
    }

    pub fn serialize(&self) -> String {
        let mut out = String::with_capacity(self.entries.len() * 32);
        for (i, e) in self.entries.iter().enumerate() {
            if i > 0 {
                out.push(RECORD_SEP);
            }
            match &e.member {
                Member::Ticket(n) => {
                    out.push(TICKET_PREFIX);
                    out.push_str(&n.to_string());
                    out.push(FIELD_SEP);
                }
                Member::User { id, name } => {
                    out.push_str(&id.to_string());
                    out.push(FIELD_SEP);
                    out.push_str(name);
                }
            }
        }
        out
    }

    /// Виклики не зберігаються: після перезавантаження всі знову чекають.
    /// Пошкоджені записи пропускає; зайве понад [`CAPACITY`] відкидає.
    pub fn deserialize(s: &str) -> Self {
        let mut q = Queue::new();
        let mut legacy_buttons = 0;
        for record in s.split(RECORD_SEP).filter(|r| !r.is_empty()) {
            let Some((id, name)) = record.split_once(FIELD_SEP) else {
                break;
            };
            if q.is_full() {
                break;
            }
            let member = if id == LEGACY_BUTTON_ID {
                // Номер видамо, коли відомі всі відновлені талони.
                legacy_buttons += 1;
                Member::Ticket(0)
            } else if let Some(n) = id.strip_prefix(TICKET_PREFIX) {
                match n.parse() {
                    Ok(n) if (1..=MAX_TICKET).contains(&n) => Member::Ticket(n),
                    _ => continue,
                }
            } else {
                match id.parse() {
                    Ok(id) => Member::user(id, name),
                    Err(_) => continue,
                }
            };
            q.entries.push_back(Entry::new(member));
        }
        q.last_ticket = q
            .entries
            .iter()
            .filter_map(|e| match e.member {
                Member::Ticket(n) => Some(n),
                Member::User { .. } => None,
            })
            .max()
            .unwrap_or(0);
        for _ in 0..legacy_buttons {
            let n = q.next_ticket_number();
            if let Some(e) = q.entries.iter_mut().find(|e| e.member == Member::Ticket(0)) {
                e.member = Member::Ticket(n);
            }
        }
        q
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const T: u64 = CALL_TIMEOUT_MS;

    fn names(q: &Queue) -> Vec<String> {
        q.iter().map(|e| e.member.display_name()).collect()
    }

    fn queue_of(ids: &[i64]) -> Queue {
        let mut q = Queue::new();
        for &id in ids {
            q.try_join(id, &format!("u{id}"));
        }
        q
    }

    #[test]
    fn join_reports_positions_and_duplicates() {
        let mut q = Queue::new();
        assert_eq!(q.try_join(1, "@a"), JoinResult::Added(1));
        assert_eq!(q.try_join(2, "@b"), JoinResult::Added(2));
        assert_eq!(q.try_join(1, "@a"), JoinResult::AlreadyIn(1));
    }

    #[test]
    fn capacity_applies_to_users_and_tickets() {
        let mut q = Queue::new();
        for i in 0..CAPACITY as i64 {
            q.try_join(i + 1, "x");
        }
        assert_eq!(q.try_join(999, "y"), JoinResult::Full);
        assert_eq!(q.take_ticket(), None);
    }

    #[test]
    fn tickets_are_sequential_and_skip_numbers_in_use() {
        let mut q = Queue::new();
        assert_eq!(q.take_ticket(), Some(1));
        assert_eq!(q.take_ticket(), Some(2));
        q.last_ticket = MAX_TICKET;
        assert_eq!(q.take_ticket(), Some(3), "після 99 по колу, пропускаючи зайняті 1 і 2");
        assert_eq!(names(&q), ["Талон №1", "Талон №2", "Талон №3"]);
    }

    #[test]
    fn calls_as_many_as_free_showers() {
        let mut q = queue_of(&[1, 2, 3]);
        let ev = q.tick(2, 0);
        assert_eq!(ev, [Event::Called(Member::user(1, "u1")), Event::Called(Member::user(2, "u2"))]);
        assert!(q.tick(2, 10).is_empty(), "повторно не кличемо");
        assert_eq!(q.get(2).unwrap().status, Status::Waiting);
    }

    #[test]
    fn fewer_free_showers_uncall_lowest_unconfirmed() {
        let mut q = queue_of(&[1, 2, 3]);
        q.tick(2, 0);
        assert!(q.confirm(2, 5));
        q.tick(1, 10);
        assert_eq!(q.get(0).unwrap().status, Status::Waiting);
        assert_eq!(q.get(1).unwrap().status, Status::Confirmed { since_ms: 5 });
    }

    #[test]
    fn timeout_moves_below_next_waiting_and_calls_next() {
        let mut q = queue_of(&[1, 2, 3]);
        q.tick(1, 0);
        let ev = q.tick(1, T);
        assert_eq!(
            ev,
            [
                Event::Moved { member: Member::user(1, "u1"), position: 2 },
                Event::Called(Member::user(2, "u2")),
            ]
        );
        assert_eq!(names(&q), ["u2", "u1", "u3"]);
        assert_eq!(q.get(1).unwrap().timeouts, 1);
    }

    #[test]
    fn removed_after_max_timeouts() {
        let mut q = queue_of(&[1]);
        let mut now = 0;
        q.tick(1, now);
        for _ in 0..MAX_TIMEOUTS - 1 {
            now += T;
            // Позаду нікого: виклик повторюється як нагадування.
            assert_eq!(q.tick(1, now), [Event::Called(Member::user(1, "u1"))]);
        }
        now += T;
        assert_eq!(q.tick(1, now), [Event::Removed(Member::user(1, "u1"))]);
        assert!(q.is_empty());
    }

    #[test]
    fn confirmation_restarts_deadline() {
        let mut q = queue_of(&[1, 2]);
        q.tick(1, 0);
        q.confirm(1, T - 1);
        assert!(q.tick(1, T).is_empty());
        let ev = q.tick(1, 2 * T - 1);
        assert_eq!(ev[0], Event::Moved { member: Member::user(1, "u1"), position: 2 }, "дедлайн від підтвердження");
    }

    #[test]
    fn confirm_requires_call() {
        let mut q = queue_of(&[1, 2]);
        q.tick(1, 0);
        assert!(!q.confirm(2, 0));
        assert!(!q.confirm(9, 0));
        assert!(q.confirm(1, 0));
    }

    #[test]
    fn voluntary_skip() {
        let mut q = queue_of(&[1, 2, 3]);
        q.tick(1, 0);
        assert_eq!(q.skip(1), SkipResult::Moved(2));
        assert_eq!(names(&q), ["u2", "u1", "u3"]);
        assert_eq!(q.get(1).unwrap().timeouts, 0, "добровільний пропуск не штрафується");
        assert_eq!(q.tick(1, 1), [Event::Called(Member::user(2, "u2"))]);
        assert_eq!(q.skip(3), SkipResult::NobodyBehind);
        assert_eq!(q.skip(9), SkipResult::NotInQueue);
    }

    #[test]
    fn skip_lets_waiting_person_past_called_ones() {
        let mut q = queue_of(&[1, 2, 3]);
        q.tick(2, 0);
        assert_eq!(q.skip(1), SkipResult::Moved(3));
        assert_eq!(names(&q), ["u2", "u3", "u1"]);
    }

    #[test]
    fn enter_takes_confirmed_first_then_called() {
        let mut q = queue_of(&[1, 2, 3]);
        q.tick(2, 0);
        q.confirm(2, 0);
        assert_eq!(q.on_enter(), Some(Member::user(2, "u2")));
        assert_eq!(q.on_enter(), Some(Member::user(1, "u1")));
        assert_eq!(q.on_enter(), None, "u3 не викликали — зайшли поза чергою");
        assert_eq!(q.len(), 1);
    }

    #[test]
    fn reset_calls_freezes_queue() {
        let mut q = queue_of(&[1, 2]);
        q.tick(2, 0);
        q.reset_calls();
        assert!(q.iter().all(|e| e.status == Status::Waiting));
        assert_eq!(q.on_enter(), None);
    }

    #[test]
    fn leave_keeps_order() {
        let mut q = queue_of(&[1, 2, 3]);
        assert_eq!(q.leave(2), Some(1));
        assert_eq!(q.leave(2), None);
        assert_eq!(names(&q), ["u1", "u3"]);
    }

    #[test]
    fn display_names() {
        assert_eq!(Member::Ticket(7).display_name(), "Талон №7");
        assert_eq!(Member::user(5, "").display_name(), "5");
        assert_eq!(Member::user(5, "@vova").display_name(), "@vova");
    }

    #[test]
    fn serialization_roundtrip_drops_calls() {
        let mut q = Queue::new();
        q.try_join(1433760480, "@vova");
        q.take_ticket();
        q.try_join(-7, "Ім'я з пробілом");
        q.try_join(8, "");
        q.tick(1, 0);
        let restored = Queue::deserialize(&q.serialize());
        assert_eq!(names(&restored), names(&q));
        assert!(restored.iter().all(|e| e.status == Status::Waiting));
        assert_eq!(Queue::deserialize(""), Queue::new());
    }

    #[test]
    fn reads_cpp_firmware_format() {
        let stored = "1433760480\x1F@vova\x1E0\x1F\x1Et5\x1F\x1E0\x1F\x1E42\x1FOlha";
        let mut q = Queue::deserialize(stored);
        assert_eq!(names(&q), ["@vova", "Талон №6", "Талон №5", "Талон №7", "Olha"]);
        assert_eq!(q.take_ticket(), Some(8));
    }

    #[test]
    fn skips_garbage_records() {
        let q = Queue::deserialize("abc\x1Fx\x1Et0\x1F\x1E5\x1Fok");
        assert_eq!(names(&q), ["ok"]);
    }
}
