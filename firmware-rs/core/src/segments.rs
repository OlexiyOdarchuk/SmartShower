//! Кодування символів для 4-розрядного 7-сегментного TM1637 з двокрапкою.

/// Біт двокрапки — старший біт другого розряду.
const COLON: u8 = 0x80;

const DIGITS: [u8; 10] = [0x3F, 0x06, 0x5B, 0x4F, 0x66, 0x6D, 0x7D, 0x07, 0x7F, 0x6F];

pub const FREE: [u8; 4] = [0x71, 0x50, 0x79, 0x79]; // F r E E

/// Таймер «ММ:СС» від початку душу; хвилини йдуть по колу через 60.
pub fn timer(elapsed_ms: u64) -> [u8; 4] {
    let secs = elapsed_ms / 1000;
    let m = (secs / 60 % 60) as usize;
    let s = (secs % 60) as usize;
    [DIGITS[m / 10], DIGITS[m % 10] | COLON, DIGITS[s / 10], DIGITS[s % 10]]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timer_formats_minutes_and_seconds() {
        assert_eq!(timer(0), [0x3F, 0x3F | COLON, 0x3F, 0x3F]);
        assert_eq!(timer(83_000), [DIGITS[0], DIGITS[1] | COLON, DIGITS[2], DIGITS[3]]);
        assert_eq!(timer(61 * 60_000), timer(60_000));
    }
}
