/// Інтервал годин, включно з обох боків. `start > finish` означає перехід
/// через північ (наприклад, 22..=5).
#[derive(Clone, Copy, Debug)]
pub struct HourRange {
    pub start: u8,
    pub finish: u8,
}

impl HourRange {
    pub const fn new(start: u8, finish: u8) -> Self {
        Self { start, finish }
    }

    pub fn contains(&self, hour: u8) -> bool {
        if self.start <= self.finish {
            hour >= self.start && hour <= self.finish
        } else {
            hour >= self.start || hour <= self.finish
        }
    }
}

/// Неробочі інтервали доби.
#[derive(Clone, Copy, Debug)]
pub struct Schedule {
    pub off: &'static [HourRange],
}

impl Schedule {
    /// `None` — час ще не синхронізовано з NTP. Тоді вважаємо час робочим,
    /// інакше до синку черга автоматично чистилась би.
    pub fn is_working(&self, local_hour: Option<u8>) -> bool {
        match local_hour {
            None => true,
            Some(h) => !self.off.iter().any(|r| r.contains(h)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DORM: Schedule = Schedule {
        off: &[HourRange::new(0, 5), HourRange::new(11, 13)],
    };

    #[test]
    fn dorm_schedule() {
        let working: Vec<u8> = (0..24).filter(|&h| DORM.is_working(Some(h))).collect();
        assert_eq!(working, [6, 7, 8, 9, 10, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23]);
    }

    #[test]
    fn unsynced_time_is_working() {
        assert!(DORM.is_working(None));
    }

    #[test]
    fn range_over_midnight() {
        let r = HourRange::new(22, 5);
        assert!(r.contains(23) && r.contains(0) && r.contains(5));
        assert!(!r.contains(6) && !r.contains(21));
    }
}
