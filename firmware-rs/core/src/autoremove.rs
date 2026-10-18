use crate::queue::Member;

/// Скільки першому в черзі дається, щоб зайти у вільний душ.
pub const TIMEOUT_MS: u64 = 5 * 60 * 1000;

/// Стежить за першим у черзі. Відлік іде лише доки є вільна кабінка: якщо
/// обидва душі зайняті, людина об'єктивно не може зайти, і виганяти її не можна.
#[derive(Debug, Default)]
pub struct AutoRemove {
    tracked: Option<i64>,
    since_ms: Option<u64>,
}

impl AutoRemove {
    /// Повертає id користувача, якого треба прибрати з голови черги.
    pub fn tick(&mut self, head: Option<&Member>, any_shower_free: bool, now_ms: u64) -> Option<i64> {
        // Немає реального «наступного» (порожньо або анонімна кнопка) — скидаємо.
        let Some(head_id) = head.and_then(Member::user_id) else {
            *self = Self::default();
            return None;
        };

        if self.tracked != Some(head_id) {
            self.tracked = Some(head_id);
            self.since_ms = any_shower_free.then_some(now_ms);
            return None;
        }

        if !any_shower_free {
            self.since_ms = None; // пауза, поки нікуди заходити
            return None;
        }
        let Some(since) = self.since_ms else {
            self.since_ms = Some(now_ms); // душ щойно звільнився — повний таймаут
            return None;
        };
        if now_ms.saturating_sub(since) < TIMEOUT_MS {
            return None;
        }

        *self = Self::default();
        Some(head_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn user(id: i64) -> Member {
        Member::user(id, "x")
    }

    #[test]
    fn removes_after_timeout_with_free_shower() {
        let mut ar = AutoRemove::default();
        let u = user(1);
        assert_eq!(ar.tick(Some(&u), true, 0), None);
        assert_eq!(ar.tick(Some(&u), true, TIMEOUT_MS - 1), None);
        assert_eq!(ar.tick(Some(&u), true, TIMEOUT_MS), Some(1));
        // Після спрацювання відлік починається з нуля.
        assert_eq!(ar.tick(Some(&u), true, TIMEOUT_MS + 1), None);
    }

    #[test]
    fn busy_showers_pause_and_reset_countdown() {
        let mut ar = AutoRemove::default();
        let u = user(1);
        ar.tick(Some(&u), true, 0);
        ar.tick(Some(&u), false, TIMEOUT_MS - 10);
        assert_eq!(ar.tick(Some(&u), true, TIMEOUT_MS), None);
        assert_eq!(ar.tick(Some(&u), true, 2 * TIMEOUT_MS - 1), None);
        assert_eq!(ar.tick(Some(&u), true, 2 * TIMEOUT_MS), Some(1));
    }

    #[test]
    fn new_head_restarts_countdown() {
        let mut ar = AutoRemove::default();
        ar.tick(Some(&user(1)), true, 0);
        assert_eq!(ar.tick(Some(&user(2)), true, TIMEOUT_MS), None);
        assert_eq!(ar.tick(Some(&user(2)), true, 2 * TIMEOUT_MS), Some(2));
    }

    #[test]
    fn button_or_empty_head_is_ignored() {
        let mut ar = AutoRemove::default();
        assert_eq!(ar.tick(Some(&Member::Button), true, 0), None);
        assert_eq!(ar.tick(Some(&Member::Button), true, 10 * TIMEOUT_MS), None);
        assert_eq!(ar.tick(None, true, 20 * TIMEOUT_MS), None);
    }
}
