/// Ловить перехід «відпущено → натиснуто».
#[derive(Debug, Default)]
pub struct Edge {
    last: bool,
}

impl Edge {
    pub fn new(initial: bool) -> Self {
        Self { last: initial }
    }

    pub fn rising(&mut self, pressed: bool) -> bool {
        let fired = pressed && !self.last;
        self.last = pressed;
        fired
    }
}

/// Кнопка з дебаунсом і мінімальним інтервалом між спрацюваннями.
#[derive(Debug)]
pub struct DebouncedButton {
    debounce_ms: u64,
    refire_ms: u64,
    raw: bool,
    stable: bool,
    changed_at: u64,
    fired_at: Option<u64>,
}

impl DebouncedButton {
    pub fn new(debounce_ms: u64, refire_ms: u64) -> Self {
        Self { debounce_ms, refire_ms, raw: false, stable: false, changed_at: 0, fired_at: None }
    }

    /// `true` — зафіксовано нове натискання.
    pub fn update(&mut self, pressed: bool, now_ms: u64) -> bool {
        if pressed != self.raw {
            self.raw = pressed;
            self.changed_at = now_ms;
            return false;
        }
        if now_ms.saturating_sub(self.changed_at) < self.debounce_ms || pressed == self.stable {
            return false;
        }
        self.stable = pressed;
        if !pressed {
            return false;
        }
        if self.fired_at.is_some_and(|t| now_ms.saturating_sub(t) < self.refire_ms) {
            return false;
        }
        self.fired_at = Some(now_ms);
        true
    }
}

/// Обмежує частоту дій: [`RateLimit::try_fire`] пропускає не частіше ніж раз на `interval_ms`.
#[derive(Debug)]
pub struct RateLimit {
    interval_ms: u64,
    last: Option<u64>,
}

impl RateLimit {
    pub fn new(interval_ms: u64) -> Self {
        Self { interval_ms, last: None }
    }

    pub fn try_fire(&mut self, now_ms: u64) -> bool {
        if self.last.is_some_and(|t| now_ms.saturating_sub(t) < self.interval_ms) {
            return false;
        }
        self.last = Some(now_ms);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edge_fires_once_per_press() {
        let mut e = Edge::new(false);
        assert!(e.rising(true));
        assert!(!e.rising(true));
        assert!(!e.rising(false));
        assert!(e.rising(true));
    }

    #[test]
    fn edge_ignores_initially_pressed() {
        let mut e = Edge::new(true);
        assert!(!e.rising(true));
    }

    #[test]
    fn debounce_filters_bounce() {
        let mut b = DebouncedButton::new(30, 300);
        assert!(!b.update(true, 0));
        assert!(!b.update(false, 5));
        assert!(!b.update(true, 10));
        assert!(!b.update(true, 39));
        assert!(b.update(true, 40));
        assert!(!b.update(true, 100));
    }

    #[test]
    fn debounce_refire_guard() {
        let mut b = DebouncedButton::new(30, 300);
        b.update(true, 0);
        assert!(b.update(true, 30));
        b.update(false, 50);
        b.update(false, 80);
        b.update(true, 100);
        assert!(!b.update(true, 130)); // занадто швидко після попереднього
        b.update(false, 200);
        b.update(false, 230);
        b.update(true, 400);
        assert!(b.update(true, 430));
    }

    #[test]
    fn rate_limit() {
        let mut r = RateLimit::new(2000);
        assert!(r.try_fire(0));
        assert!(!r.try_fire(1999));
        assert!(r.try_fire(2000));
    }
}
