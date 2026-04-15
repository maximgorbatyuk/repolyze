//! Date arithmetic utilities without chrono dependency.

/// Parse "YYYY-MM-DD" into (year, month, day).
pub fn parse_ymd(date: &str) -> Option<(i32, u32, u32)> {
    let parts: Vec<&str> = date.split('-').collect();
    if parts.len() != 3 {
        return None;
    }
    let y: i32 = parts[0].parse().ok()?;
    let m: u32 = parts[1].parse().ok()?;
    let d: u32 = parts[2].parse().ok()?;
    if !(1..=12).contains(&m) || !(1..=31).contains(&d) {
        return None;
    }
    Some((y, m, d))
}

/// Day of week: 0=Monday..6=Sunday (Sakamoto algorithm).
pub fn day_of_week(year: i32, month: u32, day: u32) -> usize {
    let t = [0, 3, 2, 5, 0, 3, 5, 1, 4, 6, 2, 4];
    let y = if month < 3 { year - 1 } else { year };
    let dow = (y + y / 4 - y / 100 + y / 400 + t[(month - 1) as usize] + day as i32) % 7;
    ((dow + 6) % 7) as usize
}

/// Format (y, m, d) as "YYYY-MM-DD".
pub fn format_ymd(y: i32, m: u32, d: u32) -> String {
    format!("{y:04}-{m:02}-{d:02}")
}

/// Convert a Gregorian date to Julian Day Number.
pub fn to_jdn(y: i32, m: u32, d: u32) -> i64 {
    let y = y as i64;
    let m = m as i64;
    let d = d as i64;
    let a = (14 - m) / 12;
    let y2 = y + 4800 - a;
    let m2 = m + 12 * a - 3;
    d + (153 * m2 + 2) / 5 + 365 * y2 + y2 / 4 - y2 / 100 + y2 / 400 - 32045
}

/// Convert a Julian Day Number back to Gregorian (y, m, d).
fn from_jdn(jdn: i64) -> (i32, u32, u32) {
    let a = jdn + 32044;
    let b = (4 * a + 3) / 146097;
    let c = a - (146097 * b) / 4;
    let d = (4 * c + 3) / 1461;
    let e = c - (1461 * d) / 4;
    let m = (5 * e + 2) / 153;
    let day = (e - (153 * m + 2) / 5 + 1) as u32;
    let month = (m + 3 - 12 * (m / 10)) as u32;
    let year = (100 * b + d - 4800 + m / 10) as i32;
    (year, month, day)
}

/// Add `n` days to a "YYYY-MM-DD" date string. `n` can be negative.
pub fn add_days(date: &str, n: i32) -> String {
    let (y, m, d) = parse_ymd(date).unwrap_or((1970, 1, 1));
    let jdn = to_jdn(y, m, d) + n as i64;
    let (y2, m2, d2) = from_jdn(jdn);
    format_ymd(y2, m2, d2)
}

/// Return the Monday of the week containing `date` as "YYYY-MM-DD", or `None` if the
/// input is not a valid "YYYY-MM-DD" date.
///
/// Weeks are Monday-starting (matches `day_of_week` convention where 0=Monday), so a
/// Sunday rolls back six days to the previous Monday.
pub fn monday_of_week(date: &str) -> Option<String> {
    let (y, m, d) = parse_ymd(date)?;
    let dow = day_of_week(y, m, d) as i32;
    Some(add_days(&format_ymd(y, m, d), -dow))
}

/// Three-letter month abbreviation.
pub fn month_abbrev(month: u32) -> &'static str {
    match month {
        1 => "Jan",
        2 => "Feb",
        3 => "Mar",
        4 => "Apr",
        5 => "May",
        6 => "Jun",
        7 => "Jul",
        8 => "Aug",
        9 => "Sep",
        10 => "Oct",
        11 => "Nov",
        12 => "Dec",
        _ => "???",
    }
}

/// Format a Unix timestamp (seconds since epoch) as "YYYY-MM-DD".
pub fn format_unix_timestamp(secs: u64) -> String {
    let days_since_epoch = (secs / 86400) as i64;
    // 1970-01-01 is JDN 2440588
    let jdn = 2_440_588 + days_since_epoch;
    let (y, m, d) = from_jdn(jdn);
    format_ymd(y, m, d)
}

/// Current date as "YYYY-MM-DD" from system time.
pub fn today_ymd() -> String {
    // Use UNIX_EPOCH + SystemTime to get current UTC date
    let dur = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let days_since_epoch = (dur.as_secs() / 86400) as i64;
    // 1970-01-01 is JDN 2440588
    let jdn = 2_440_588 + days_since_epoch;
    let (y, m, d) = from_jdn(jdn);
    format_ymd(y, m, d)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_ymd_valid() {
        assert_eq!(parse_ymd("2025-01-15"), Some((2025, 1, 15)));
        assert_eq!(parse_ymd("2000-12-31"), Some((2000, 12, 31)));
    }

    #[test]
    fn parse_ymd_invalid() {
        assert!(parse_ymd("not-a-date").is_none());
        assert!(parse_ymd("2025-13-01").is_none());
        assert!(parse_ymd("2025-00-01").is_none());
        assert!(parse_ymd("2025-01-00").is_none());
    }

    #[test]
    fn day_of_week_known_dates() {
        // 2025-01-13 is Monday (0)
        assert_eq!(day_of_week(2025, 1, 13), 0);
        // 2025-01-15 is Wednesday (2)
        assert_eq!(day_of_week(2025, 1, 15), 2);
        // 2025-01-19 is Sunday (6)
        assert_eq!(day_of_week(2025, 1, 19), 6);
        // 2024-02-29 is Thursday (3) — leap year
        assert_eq!(day_of_week(2024, 2, 29), 3);
    }

    #[test]
    fn add_days_forward() {
        assert_eq!(add_days("2025-01-30", 2), "2025-02-01");
        assert_eq!(add_days("2025-12-31", 1), "2026-01-01");
    }

    #[test]
    fn add_days_backward() {
        assert_eq!(add_days("2025-01-01", -1), "2024-12-31");
        assert_eq!(add_days("2025-03-01", -1), "2025-02-28");
    }

    #[test]
    fn add_days_leap_year() {
        assert_eq!(add_days("2024-02-28", 1), "2024-02-29");
        assert_eq!(add_days("2024-02-29", 1), "2024-03-01");
        // Non-leap year
        assert_eq!(add_days("2025-02-28", 1), "2025-03-01");
    }

    #[test]
    fn format_ymd_pads_correctly() {
        assert_eq!(format_ymd(2025, 1, 5), "2025-01-05");
        assert_eq!(format_ymd(2025, 12, 31), "2025-12-31");
    }

    #[test]
    fn month_abbrev_all() {
        assert_eq!(month_abbrev(1), "Jan");
        assert_eq!(month_abbrev(6), "Jun");
        assert_eq!(month_abbrev(12), "Dec");
        assert_eq!(month_abbrev(0), "???");
    }

    #[test]
    fn format_unix_timestamp_known_dates() {
        assert_eq!(format_unix_timestamp(0), "1970-01-01");
        assert_eq!(format_unix_timestamp(1704067200), "2024-01-01");
        assert_eq!(format_unix_timestamp(1710720000), "2024-03-18");
    }

    #[test]
    fn monday_of_week_returns_same_date_for_monday() {
        // 2025-01-13 is a Monday
        assert_eq!(monday_of_week("2025-01-13").as_deref(), Some("2025-01-13"));
    }

    #[test]
    fn monday_of_week_wednesday_rolls_back() {
        // 2025-01-15 is a Wednesday → Monday 2025-01-13
        assert_eq!(monday_of_week("2025-01-15").as_deref(), Some("2025-01-13"));
    }

    #[test]
    fn monday_of_week_sunday_rolls_back_six_days() {
        // 2025-01-19 is a Sunday → Monday 2025-01-13
        assert_eq!(monday_of_week("2025-01-19").as_deref(), Some("2025-01-13"));
    }

    #[test]
    fn monday_of_week_crosses_month_boundary() {
        // 2025-03-01 is a Saturday → Monday 2025-02-24
        assert_eq!(monday_of_week("2025-03-01").as_deref(), Some("2025-02-24"));
    }

    #[test]
    fn monday_of_week_invalid_returns_none() {
        assert!(monday_of_week("not-a-date").is_none());
        assert!(monday_of_week("2025-13-01").is_none());
        assert!(monday_of_week("").is_none());
    }

    #[test]
    fn today_ymd_format() {
        let today = today_ymd();
        assert_eq!(today.len(), 10);
        assert!(parse_ymd(&today).is_some());
    }
}
