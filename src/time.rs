//! Human-friendly times without pulling in a date crate. Times are Unix milliseconds;
//! calendar dates are UTC.

use std::time::{SystemTime, UNIX_EPOCH};

pub fn now_millis() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_millis() as u64)
}

/// "just now", "5 min ago", "3 h ago", "yesterday", "4 days ago", "12 Oct".
pub fn ago(millis: u64) -> String {
    if millis == 0 {
        return String::new();
    }
    let secs = now_millis().saturating_sub(millis) / 1000;
    match secs {
        0..=59 => "just now".into(),
        60..=3599 => format!("{} min ago", secs / 60),
        3600..=86_399 => format!("{} h ago", secs / 3600),
        86_400..=172_799 => "yesterday".into(),
        _ if secs < 7 * 86_400 => format!("{} days ago", secs / 86_400),
        _ => date(millis),
    }
}

/// "12 Oct", or "12 Oct 2024" for another year.
pub fn date(millis: u64) -> String {
    const MONTHS: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
    let (y, m, d) = civil(millis / 86_400_000);
    let (now_y, _, _) = civil(now_millis() / 86_400_000);
    if y == now_y {
        format!("{d} {}", MONTHS[m as usize - 1])
    } else {
        format!("{d} {} {y}", MONTHS[m as usize - 1])
    }
}

/// "20261014-160633", for file names.
pub fn stamp(millis: u64) -> String {
    let (y, m, d) = civil(millis / 86_400_000);
    let secs = (millis / 1000) % 86_400;
    format!("{y:04}{m:02}{d:02}-{:02}{:02}{:02}", secs / 3600, secs / 60 % 60, secs % 60)
}

/// Days since 1970-01-01 to (year, month, day). Howard Hinnant's algorithm.
fn civil(days: u64) -> (i64, u32, u32) {
    let z = days as i64 + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let y = yoe + era * 400 + i64::from(m <= 2);
    (y, m, d)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calendar() {
        assert_eq!(civil(0), (1970, 1, 1));
        assert_eq!(civil(20_375), (2025, 10, 14));
        assert_eq!(stamp(1_760_457_993_000), "20251014-160633");
        assert_eq!(ago(0), "");
        assert_eq!(ago(now_millis() - 5 * 60_000), "5 min ago");
    }
}
