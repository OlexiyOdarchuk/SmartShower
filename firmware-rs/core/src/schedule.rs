/// Інтервал годин, включно з обох боків. `start > finish` означає перехід
/// через північ (наприклад, 22..=5).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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

    /// Година, з якої інтервал закінчився.
    pub fn end_hour(&self) -> u8 {
        (self.finish + 1) % 24
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Open,
    /// Черга заморожена: викликів немає, ставати можна.
    Break { until_hour: u8 },
    /// Черга закрита й очищена.
    Night { until_hour: u8 },
}

#[derive(Clone, Copy, Debug)]
pub struct Schedule {
    pub night: HourRange,
    pub breaks: &'static [HourRange],
}

impl Schedule {
    /// `None` — час ще не синхронізовано з NTP. Тоді вважаємо час робочим,
    /// інакше до синку черга автоматично чистилась би.
    pub fn mode(&self, local_hour: Option<u8>) -> Mode {
        let Some(h) = local_hour else {
            return Mode::Open;
        };
        if self.night.contains(h) {
            return Mode::Night { until_hour: self.night.end_hour() };
        }
        match self.breaks.iter().find(|r| r.contains(h)) {
            Some(r) => Mode::Break { until_hour: r.end_hour() },
            None => Mode::Open,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DORM: Schedule = Schedule { night: HourRange::new(0, 5), breaks: &[HourRange::new(11, 13)] };

    #[test]
    fn dorm_schedule() {
        assert_eq!(DORM.mode(Some(3)), Mode::Night { until_hour: 6 });
        assert_eq!(DORM.mode(Some(6)), Mode::Open);
        assert_eq!(DORM.mode(Some(12)), Mode::Break { until_hour: 14 });
        assert_eq!(DORM.mode(Some(14)), Mode::Open);
        assert_eq!(DORM.mode(Some(23)), Mode::Open);
    }

    #[test]
    fn unsynced_time_is_open() {
        assert_eq!(DORM.mode(None), Mode::Open);
    }

    #[test]
    fn range_over_midnight() {
        let r = HourRange::new(22, 5);
        assert!(r.contains(23) && r.contains(0) && r.contains(5));
        assert!(!r.contains(6) && !r.contains(21));
        assert_eq!(r.end_hour(), 6);
        assert_eq!(HourRange::new(20, 23).end_hour(), 0);
    }
}
