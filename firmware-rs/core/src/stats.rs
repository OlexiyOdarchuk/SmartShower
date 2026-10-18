//! Середня тривалість душу й оцінка часу очікування.

pub const DEFAULT_SHOWER_MS: u64 = 15 * 60 * 1000;
/// Коротші сесії — це двері, відчинені помилково; довші — забута кнопка.
const MIN_SESSION_MS: u64 = 2 * 60 * 1000;
const MAX_SESSION_MS: u64 = 60 * 60 * 1000;
/// Навіть якщо душ триває довше за середнє, звільниться він не миттєво.
const MIN_REMAINING_MS: u64 = 60 * 1000;

/// Експоненційне середнє тривалості душу.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ShowerStats {
    avg_ms: u64,
}

impl Default for ShowerStats {
    fn default() -> Self {
        Self { avg_ms: DEFAULT_SHOWER_MS }
    }
}

impl ShowerStats {
    pub fn new(avg_ms: u64) -> Self {
        Self { avg_ms: avg_ms.clamp(MIN_SESSION_MS, MAX_SESSION_MS) }
    }

    pub fn avg_ms(&self) -> u64 {
        self.avg_ms
    }

    /// `false` — сесію відкинуто як неправдоподібну.
    pub fn record(&mut self, duration_ms: u64) -> bool {
        if !(MIN_SESSION_MS..=MAX_SESSION_MS).contains(&duration_ms) {
            return false;
        }
        self.avg_ms = (self.avg_ms * 4 + duration_ms) / 5;
        true
    }

    /// Скільки чекати людині, перед якою `ahead` людей.
    /// `busy_for[i]` — скільки вже триває душ у кабінці i (`None` — вільна).
    pub fn eta_ms(&self, ahead: usize, busy_for: &[Option<u64>]) -> u64 {
        let mut slots: Vec<u64> = busy_for
            .iter()
            .map(|b| b.map_or(0, |elapsed| self.avg_ms.saturating_sub(elapsed).max(MIN_REMAINING_MS)))
            .collect();
        if slots.is_empty() {
            return 0;
        }
        for _ in 0..ahead {
            let earliest = slots.iter_mut().min().expect("слоти не порожні");
            *earliest += self.avg_ms;
        }
        slots.into_iter().min().unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MIN: u64 = 60 * 1000;

    #[test]
    fn records_plausible_sessions_only() {
        let mut s = ShowerStats::new(10 * MIN);
        assert!(!s.record(MIN));
        assert!(!s.record(3 * 60 * MIN));
        assert!(s.record(20 * MIN));
        assert_eq!(s.avg_ms(), 12 * MIN);
    }

    #[test]
    fn eta_with_free_showers() {
        let s = ShowerStats::new(10 * MIN);
        assert_eq!(s.eta_ms(0, &[None, None]), 0);
        assert_eq!(s.eta_ms(1, &[None, None]), 0);
        assert_eq!(s.eta_ms(2, &[None, None]), 10 * MIN);
        assert_eq!(s.eta_ms(3, &[None, None]), 10 * MIN);
        assert_eq!(s.eta_ms(4, &[None, None]), 20 * MIN);
    }

    #[test]
    fn eta_counts_remaining_time_of_busy_showers() {
        let s = ShowerStats::new(10 * MIN);
        assert_eq!(s.eta_ms(0, &[Some(4 * MIN), Some(8 * MIN)]), 2 * MIN);
        assert_eq!(s.eta_ms(1, &[Some(4 * MIN), Some(8 * MIN)]), 6 * MIN);
        assert_eq!(s.eta_ms(0, &[Some(30 * MIN), None]), 0);
        assert_eq!(s.eta_ms(0, &[Some(30 * MIN), Some(30 * MIN)]), MIN);
    }

    #[test]
    fn new_clamps_garbage() {
        assert_eq!(ShowerStats::new(0).avg_ms(), MIN_SESSION_MS);
    }
}
