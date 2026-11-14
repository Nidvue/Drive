// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Nidvue

//! Time helpers. Deliberately dependency-free: one civil-date algorithm,
//! unit-tested, instead of a calendar crate.

use std::time::{SystemTime, UNIX_EPOCH};

use crate::error::{NidvueError, Result};

/// Milliseconds since the Unix epoch, or 0 if the clock is before 1970.
pub fn now_ms() -> i64 {
    match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(d) => d.as_millis() as i64,
        Err(_) => 0,
    }
}

/// Convert milliseconds since the epoch to a civil UTC date and time.
///
/// Returns `(year, month, day, hour, minute, second)`.
pub fn civil_from_ms(ms: i64) -> (i64, u32, u32, u32, u32, u32) {
    let secs = ms.div_euclid(1000);
    let days = secs.div_euclid(86_400);
    let rem = secs.rem_euclid(86_400);
    let (y, m, d) = civil_from_days(days);
    (
        y,
        m,
        d,
        (rem / 3600) as u32,
        ((rem % 3600) / 60) as u32,
        (rem % 60) as u32,
    )
}

/// Howard Hinnant's `civil_from_days`, valid for the whole proleptic Gregorian
/// calendar.
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64; // [0, 146096]
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365; // [0, 399]
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // [0, 365]
    let mp = (5 * doy + 2) / 153; // [0, 11]
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32; // [1, 31]
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32; // [1, 12]
    (if m <= 2 { y + 1 } else { y }, m, d)
}

/// `YYYY-MM-DD HHMMSS`, the timestamp used in conflict copy names.
pub fn conflict_timestamp(ms: i64) -> String {
    let (y, mo, d, h, mi, s) = civil_from_ms(ms);
    format!("{y:04}-{mo:02}-{d:02} {h:02}{mi:02}{s:02}")
}

/// `YYYY-MM-DDTHH:MM:SSZ`, used in logs and JSON output.
pub fn iso8601(ms: i64) -> String {
    let (y, mo, d, h, mi, s) = civil_from_ms(ms);
    format!("{y:04}-{mo:02}-{d:02}T{h:02}:{mi:02}:{s:02}Z")
}

/// Parse the Jottacloud timestamp forms.
///
/// JFS historically used `2018-07-24-T20:41:10Z` (note the extra dash) while
/// newer endpoints use RFC 3339, sometimes with milliseconds.
pub fn parse_jotta_time(input: &str) -> Result<i64> {
    let s = input.trim();
    if s.is_empty() {
        return Err(NidvueError::Internal("empty timestamp".into()));
    }
    // Normalise: drop the historical `-T` dash and any fractional seconds.
    let s = s.replacen("-T", "T", 1);
    let (date, rest) = s
        .split_once('T')
        .ok_or_else(|| NidvueError::Internal(format!("bad timestamp: {input}")))?;
    let mut rest = rest.to_string();
    // Offset first, then fractional seconds: `10.5+0200` and `10.5Z` both occur.
    let offset_secs = if let Some(stripped) = rest.strip_suffix('Z') {
        rest = stripped.to_string();
        0
    } else if rest.len() >= 5 && (rest.as_bytes()[rest.len() - 5] == b'+' || rest.as_bytes()[rest.len() - 5] == b'-') {
        let off = rest.split_off(rest.len() - 5);
        let sign = if off.starts_with('-') { -1i64 } else { 1i64 };
        let hh: i64 = off[1..3]
            .parse()
            .map_err(|_| NidvueError::Internal(format!("bad offset: {input}")))?;
        let mm: i64 = off[3..5]
            .parse()
            .map_err(|_| NidvueError::Internal(format!("bad offset: {input}")))?;
        sign * (hh * 3600 + mm * 60)
    } else {
        0
    };
    if let Some(dot) = rest.find('.') {
        rest.truncate(dot);
    }

    let mut dparts = date.split('-');
    let y: i64 = part(&mut dparts, input)?;
    let mo: i64 = part(&mut dparts, input)?;
    let d: i64 = part(&mut dparts, input)?;
    let mut tparts = rest.split(':');
    let h: i64 = part(&mut tparts, input)?;
    let mi: i64 = part(&mut tparts, input)?;
    let sec: i64 = part(&mut tparts, input)?;

    let days = days_from_civil(y, mo as u32, d as u32);
    Ok((days * 86_400 + h * 3600 + mi * 60 + sec - offset_secs) * 1000)
}

fn part<'a>(it: &mut impl Iterator<Item = &'a str>, whole: &str) -> Result<i64> {
    it.next()
        .and_then(|p| p.parse().ok())
        .ok_or_else(|| NidvueError::Internal(format!("bad timestamp: {whole}")))
}

/// Inverse of [`civil_from_days`].
fn days_from_civil(y: i64, m: u32, d: u32) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = (y - era * 400) as u64; // [0, 399]
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) as u64 + 2) / 5 + d as u64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe as i64 - 719_468
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn epoch_and_known_dates() {
        assert_eq!(civil_from_ms(0), (1970, 1, 1, 0, 0, 0));
        assert_eq!(conflict_timestamp(0), "1970-01-01 000000");
        // 2026-11-14T10:15:00Z
        let ms = parse_jotta_time("2026-11-14T10:15:00Z").unwrap();
        assert_eq!(conflict_timestamp(ms), "2026-11-14 101500");
        assert_eq!(iso8601(ms), "2026-11-14T10:15:00Z");
    }

    #[test]
    fn parses_jotta_forms() {
        let a = parse_jotta_time("2018-07-24-T20:41:10Z").unwrap();
        let b = parse_jotta_time("2018-07-24T20:41:10.123Z").unwrap();
        assert_eq!(a, b);
        let c = parse_jotta_time("2018-07-24T22:41:10+0200").unwrap();
        assert_eq!(a, c);
        assert!(parse_jotta_time("nope").is_err());
        assert!(parse_jotta_time("").is_err());
    }

    #[test]
    fn round_trips_civil_dates() {
        for days in [-100_000i64, -1, 0, 1, 19_000, 100_000] {
            let (y, m, d) = civil_from_days(days);
            assert_eq!(days_from_civil(y, m, d), days);
        }
    }

    #[test]
    fn now_is_after_2024() {
        assert!(now_ms() > 1_700_000_000_000);
    }
}
