//! The local date and time for Lua scenes. The standard library has no time zones, so the offset
//! from UTC is asked of the system once, the same way the quiet hours ask for the time. A
//! background thread keeps it fresh, so a change to or from summer time is picked up within
//! minutes.
//! The calendar arithmetic is done here, which keeps the clock free of new dependencies.

use std::cell::Cell;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicI64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// The fixed moment a snapshot's clock starts from unless `--clock` names another: Monday
/// 1 June 2026 at 09:00, so stills and tests come out the same on every machine.
pub const SNAPSHOT_START: i64 = 1_780_304_400;

const WEEKDAYS: [&str; 7] = [
    "Monday",
    "Tuesday",
    "Wednesday",
    "Thursday",
    "Friday",
    "Saturday",
    "Sunday",
];
const MONTHS: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];

/// A wall-clock date and time, already in the local time zone.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Civil {
    pub year: i64,
    /// 1 for January to 12 for December.
    pub month: u32,
    pub day: u32,
    pub hour: u32,
    pub minute: u32,
    pub second: u32,
    /// 1 for Monday to 7 for Sunday, as ISO 8601 counts them.
    pub weekday: u32,
}

impl Civil {
    /// The date and time `seconds` after 1970-01-01 00:00 on the local wall clock.
    pub fn from_local_seconds(seconds: i64) -> Civil {
        let days = seconds.div_euclid(86_400);
        let of_day = seconds.rem_euclid(86_400) as u32;
        let (year, month, day) = date_from_days(days);
        // 1970-01-01 was a Thursday, the fourth day of an ISO week
        let weekday = ((days + 3).rem_euclid(7) + 1) as u32;
        Civil {
            year,
            month,
            day,
            hour: of_day / 3600,
            minute: of_day / 60 % 60,
            second: of_day % 60,
            weekday,
        }
    }

    pub fn weekday_name(&self) -> &'static str {
        WEEKDAYS[(self.weekday as usize).clamp(1, 7) - 1]
    }

    pub fn month_name(&self) -> &'static str {
        MONTHS[(self.month as usize).clamp(1, 12) - 1]
    }

    /// The time as `14:05`.
    pub fn time_text(&self) -> String {
        format!("{:02}:{:02}", self.hour, self.minute)
    }

    /// The date as `Monday 1 June 2026`.
    pub fn date_text(&self) -> String {
        format!(
            "{} {} {} {}",
            self.weekday_name(),
            self.day,
            self.month_name(),
            self.year
        )
    }
}

/// Year, month and day for a count of days from 1970-01-01, from Howard Hinnant's civil
/// calendar algorithm, which holds for every proleptic Gregorian date.
fn date_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let month = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let year = yoe + era * 400 + i64::from(month <= 2);
    (year, month, day)
}

/// Days from 1970-01-01 for a date, the inverse of `date_from_days`.
fn days_from_date(year: i64, month: u32, day: u32) -> i64 {
    let y = if month <= 2 { year - 1 } else { year };
    let era = y.div_euclid(400);
    let yoe = y.rem_euclid(400);
    let m = i64::from(month);
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + i64::from(day) - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// Parses `--clock` for a snapshot: `HH:MM` on the default snapshot day or `YYYY-MM-DDTHH:MM`
/// (a space works in place of the `T`). Returns local seconds from 1970 or the reason the text
/// was refused.
pub fn parse_snapshot_clock(text: &str) -> Result<i64, String> {
    let text = text.trim();
    let shape = || format!("--clock wants HH:MM or YYYY-MM-DDTHH:MM, not \"{text}\"");
    let time = |part: &str| -> Option<i64> {
        let (h, m) = part.split_once(':')?;
        let h: i64 = h.parse().ok()?;
        let m: i64 = m.parse().ok()?;
        ((0..24).contains(&h) && (0..60).contains(&m)).then_some(h * 3600 + m * 60)
    };
    if let Some((date, clock)) = text.split_once(['T', ' ']) {
        let date = || -> Option<(i64, u32, u32)> {
            let mut parts = date.split('-');
            let year: i64 = parts.next()?.parse().ok()?;
            let month: u32 = parts.next()?.parse().ok()?;
            let day: u32 = parts.next()?.parse().ok()?;
            parts.next().is_none().then_some((year, month, day))
        };
        let (year, month, day) = date().ok_or_else(shape)?;
        // the dates a scene shows are four-digit years; far outside them the day sums would
        // also run past what a timestamp can hold
        if !(1..=9999).contains(&year) {
            return Err(format!(
                "--clock wants a year from 1 to 9999, not {year} in \"{text}\""
            ));
        }
        if !(1..=12).contains(&month) || day == 0 {
            return Err(shape());
        }
        let days = days_from_date(year, month, day);
        // a day past the end of its month would roll into the next one; refuse it instead
        if date_from_days(days) != (year, month, day) {
            return Err(format!(
                "--clock has no day {day} in month {month} of {year}"
            ));
        }
        return Ok(days * 86_400 + time(clock).ok_or_else(shape)?);
    }
    let midnight = SNAPSHOT_START - SNAPSHOT_START.rem_euclid(86_400);
    Ok(midnight + time(text).ok_or_else(shape)?)
}

/// The offset from UTC in seconds, asked of the system: `+0100` from `date` on macOS and Linux,
/// minutes from PowerShell on Windows.
fn ask_offset() -> Option<i64> {
    if cfg!(windows) {
        let text = crate::motion::command_output(
            "powershell",
            &[
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                "[int][TimeZoneInfo]::Local.GetUtcOffset([DateTime]::Now).TotalMinutes",
            ],
        )?;
        Some(text.trim().parse::<i64>().ok()? * 60)
    } else {
        parse_offset(&crate::motion::command_output("date", &["+%z"])?)
    }
}

/// Seconds east of UTC from `+0100`, `-0530` or `+00:00`.
fn parse_offset(text: &str) -> Option<i64> {
    let text: String = text.trim().chars().filter(|c| *c != ':').collect();
    let (sign, digits) = match text.as_bytes().first()? {
        b'+' => (1, &text[1..]),
        b'-' => (-1, &text[1..]),
        _ => return None,
    };
    if digits.len() != 4 || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let hours: i64 = digits[..2].parse().ok()?;
    let minutes: i64 = digits[2..].parse().ok()?;
    (hours <= 14 && minutes < 60).then_some(sign * (hours * 3600 + minutes * 60))
}

/// No offset known yet or the system could not say.
const UNKNOWN: i64 = i64::MIN;

/// The offset, asked once on first use and then every ten minutes on a background thread.
fn offset() -> Option<i64> {
    static OFFSET: AtomicI64 = AtomicI64::new(UNKNOWN);
    static STARTED: OnceLock<()> = OnceLock::new();
    STARTED.get_or_init(|| {
        OFFSET.store(ask_offset().unwrap_or(UNKNOWN), Ordering::Relaxed);
        std::thread::spawn(|| {
            loop {
                std::thread::sleep(Duration::from_secs(600));
                if let Some(seconds) = ask_offset() {
                    OFFSET.store(seconds, Ordering::Relaxed);
                }
            }
        });
    });
    let seconds = OFFSET.load(Ordering::Relaxed);
    (seconds != UNKNOWN).then_some(seconds)
}

/// Where a scene's clock reads from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Source {
    /// The system clock in the local time zone.
    Live,
    /// A fixed start in local seconds, moved on by the scene's own time, for snapshots and tests.
    Fixed(i64),
}

thread_local! {
    static FIXED: Cell<Option<i64>> = const { Cell::new(None) };
}

/// Runs `f` with every scene built on this thread reading a fixed clock that starts at `start`.
/// A snapshot builds its scene this way so the same `--time` always shows the same time.
pub fn with_fixed<T>(start: i64, f: impl FnOnce() -> T) -> T {
    let before = FIXED.with(|c| c.replace(Some(start)));
    let out = f();
    FIXED.with(|c| c.set(before));
    out
}

impl Source {
    /// The source a scene built now on this thread should use.
    pub fn current() -> Source {
        FIXED.with(Cell::get).map_or(Source::Live, Source::Fixed)
    }

    /// The local date and time `elapsed` seconds into the scene, `None` when the time zone is
    /// unknown, so a scene never shows a time that might be hours out.
    pub fn now(self, elapsed: f64) -> Option<Civil> {
        match self {
            Source::Fixed(start) => Some(Civil::from_local_seconds(
                start + elapsed.max(0.0).floor() as i64,
            )),
            Source::Live => {
                let utc = SystemTime::now().duration_since(UNIX_EPOCH).ok()?.as_secs();
                let utc = i64::try_from(utc).ok()?;
                Some(Civil::from_local_seconds(utc + offset()?))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_snapshot_start_is_a_monday_morning_in_june() {
        let c = Civil::from_local_seconds(SNAPSHOT_START);
        assert_eq!((c.year, c.month, c.day), (2026, 6, 1));
        assert_eq!((c.hour, c.minute, c.second), (9, 0, 0));
        assert_eq!(c.weekday_name(), "Monday");
        assert_eq!(c.date_text(), "Monday 1 June 2026");
        assert_eq!(c.time_text(), "09:00");
    }

    #[test]
    fn dates_round_trip_across_leap_years_and_centuries() {
        for (y, m, d, weekday) in [
            (1970, 1, 1, 4),
            (2000, 2, 29, 2),
            (2024, 12, 31, 2),
            (2026, 11, 7, 6),
            (2100, 3, 1, 1),
            (1969, 12, 31, 3),
        ] {
            let days = days_from_date(y, m, d);
            assert_eq!(date_from_days(days), (y, m, d));
            let c = Civil::from_local_seconds(days * 86_400 + 45_296);
            assert_eq!((c.year, c.month, c.day), (y, m, d));
            assert_eq!((c.hour, c.minute, c.second), (12, 34, 56));
            assert_eq!(c.weekday, weekday, "{y}-{m}-{d}");
        }
    }

    #[test]
    fn offsets_parse_in_every_form_the_systems_print() {
        assert_eq!(parse_offset("+0000\n"), Some(0));
        assert_eq!(parse_offset("+0100"), Some(3600));
        assert_eq!(parse_offset("-0530"), Some(-19_800));
        assert_eq!(parse_offset("+05:45"), Some(20_700));
        for bad in ["", "0100", "+01", "+2500", "+01x0"] {
            assert_eq!(parse_offset(bad), None, "{bad}");
        }
    }

    #[test]
    fn snapshot_clocks_parse_a_time_or_a_full_date() {
        let evening = parse_snapshot_clock("19:30").unwrap();
        let c = Civil::from_local_seconds(evening);
        assert_eq!((c.day, c.hour, c.minute), (1, 19, 30));
        let c = Civil::from_local_seconds(parse_snapshot_clock("2026-11-07T23:15").unwrap());
        assert_eq!(c.date_text(), "Saturday 7 November 2026");
        assert_eq!(c.time_text(), "23:15");
        assert!(parse_snapshot_clock("2026-11-07 06:00").is_ok());
        for bad in ["24:00", "9", "2026-02-30T10:00", "2026-13-01T10:00", "soon"] {
            assert!(parse_snapshot_clock(bad).is_err(), "{bad}");
        }
        assert!(parse_snapshot_clock("0001-01-01T00:00").is_ok());
        assert!(parse_snapshot_clock("9999-12-31T23:59").is_ok());
        for bad in [
            "0000-06-01T10:00",
            "10000-01-01T10:00",
            "99999999999999-01-01T10:00",
        ] {
            let error = parse_snapshot_clock(bad).unwrap_err();
            assert!(error.contains("year from 1 to 9999"), "{bad}: {error}");
        }
    }

    #[test]
    fn a_fixed_clock_moves_on_with_the_scene_and_is_scoped_to_its_closure() {
        assert_eq!(Source::current(), Source::Live);
        let inside = with_fixed(SNAPSHOT_START, Source::current);
        assert_eq!(inside, Source::Fixed(SNAPSHOT_START));
        assert_eq!(Source::current(), Source::Live);
        let later = inside.now(125.7).unwrap();
        assert_eq!((later.hour, later.minute, later.second), (9, 2, 5));
    }
}
