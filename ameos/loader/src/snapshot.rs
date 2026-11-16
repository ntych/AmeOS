use std::io;
#[cfg(target_os = "freebsd")]
use std::process::Command;

pub struct SnapshotInfo {
    pub build_number: String,
    pub release_date: String,
    pub moscow_time: String,
}

impl SnapshotInfo {
    pub fn current() -> io::Result<Self> {
        let build_number = Self::get_build_number()?;
        let release_date = Self::get_release_date()?;
        let moscow_time = Self::get_moscow_time()?;

        Ok(Self {
            build_number,
            release_date,
            moscow_time,
        })
    }

    fn get_build_number() -> io::Result<String> {
        #[cfg(target_os = "freebsd")]
        {
            let output = Command::new("/usr/bin/uname").arg("-K").output()?;

            if output.status.success() {
                let kernel_version = String::from_utf8_lossy(&output.stdout);
                Ok(kernel_version.trim().to_owned())
            } else {
                Ok("unknown".to_owned())
            }
        }

        #[cfg(not(target_os = "freebsd"))]
        {
            Ok("16.0-CURRENT-20261113".to_owned())
        }
    }

    fn get_release_date() -> io::Result<String> {
        #[cfg(target_os = "freebsd")]
        {
            let output = Command::new("/bin/date")
                .arg("-r")
                .arg("/boot/kernel/kernel")
                .arg("+%Y-%m-%d %H:%M:%S")
                .output()?;

            if output.status.success() {
                let date = String::from_utf8_lossy(&output.stdout);
                Ok(date.trim().to_owned())
            } else {
                Ok("unknown".to_owned())
            }
        }

        #[cfg(not(target_os = "freebsd"))]
        {
            Ok("2026-11-13 15:42:08".to_owned())
        }
    }

    fn get_moscow_time() -> io::Result<String> {
        #[cfg(target_os = "freebsd")]
        {
            let output = Command::new("/bin/date")
                .env("TZ", "Europe/Moscow")
                .arg("+%Y-%m-%d %H:%M:%S %Z")
                .output()?;

            if output.status.success() {
                let time = String::from_utf8_lossy(&output.stdout);
                Ok(time.trim().to_owned())
            } else {
                Ok("unknown".to_owned())
            }
        }

        #[cfg(not(target_os = "freebsd"))]
        {
            use std::time::{SystemTime, UNIX_EPOCH};
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            Ok(utc_secs_to_moscow_string(now))
        }
    }

    pub fn display(&self) -> String {
        format!(
            "AmeOS Snapshot Information\n\
             ===========================\n\
             \n\
             Build number:    {}\n\
             Release date:    {}\n\
             Moscow time:     {}\n",
            self.build_number, self.release_date, self.moscow_time
        )
    }
}

/// Convert a UTC Unix timestamp to a Moscow time string (UTC+3).
/// Pure function — no I/O, deterministically testable.
#[allow(dead_code)]
pub fn utc_secs_to_moscow_string(utc_secs: u64) -> String {
    const MOSCOW_OFFSET: u64 = 3 * 3600;
    let moscow_secs = utc_secs + MOSCOW_OFFSET;

    let days_since_epoch = moscow_secs / 86400;

    let mut year: u64 = 1970;
    let mut days = days_since_epoch;
    loop {
        let days_in_year = if is_leap_year(year) { 366 } else { 365 };
        if days < days_in_year {
            break;
        }
        days -= days_in_year;
        year += 1;
    }

    let months: [u64; 12] = if is_leap_year(year) {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };

    let mut month = 0usize;
    let mut day = days + 1;
    for (i, &days_in_month) in months.iter().enumerate() {
        if day <= days_in_month {
            month = i + 1;
            break;
        }
        day -= days_in_month;
    }

    // Defensive: the year loop guarantees days < 365 or 366, so the month
    // loop must always converge. If it somehow did not, month stays 0 and
    // the formatted date would be malformed (YYYY-00-DD).
    debug_assert!(month != 0, "month loop did not converge");

    let seconds_today = moscow_secs % 86400;
    let hour = seconds_today / 3600;
    let minute = (seconds_today % 3600) / 60;
    let second = seconds_today % 60;

    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}:{:02} MSK",
        year, month, day, hour, minute, second
    )
}

#[allow(dead_code)]
pub fn is_leap_year(year: u64) -> bool {
    (year.is_multiple_of(4) && !year.is_multiple_of(100)) || year.is_multiple_of(400)
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── is_leap_year ──────────────────────────────────────────────────────────

    #[test]
    fn leap_year_divisible_by_4() {
        assert!(is_leap_year(2024));
        assert!(is_leap_year(2000)); // divisible by 400
        assert!(is_leap_year(1600));
    }

    #[test]
    fn not_leap_year_century_not_400() {
        assert!(!is_leap_year(1900));
        assert!(!is_leap_year(2100));
        assert!(!is_leap_year(2200));
    }

    #[test]
    fn not_leap_year_odd() {
        assert!(!is_leap_year(2023));
        assert!(!is_leap_year(2025));
    }

    // ── utc_secs_to_moscow_string ─────────────────────────────────────────────

    #[test]
    fn unix_epoch_plus_offset_is_1970_01_01_03_00_00() {
        // UTC 1970-01-01 00:00:00 → MSK 1970-01-01 03:00:00
        assert_eq!(utc_secs_to_moscow_string(0), "1970-01-01 03:00:00 MSK");
    }

    #[test]
    fn known_timestamp_2026_11_15_noon_utc() {
        // 2026-11-15 12:00:00 UTC → 2026-11-15 15:00:00 MSK
        // days from epoch to 2026-11-15:
        //   1970..2025 = 56 years, 14 leap years → 56*365+14 = 20454 days (2026-01-01 = day 20454)
        //   Jan31+Feb28+Mar31+Apr30+May31+Jun30+Jul31+Aug31+Sep30+Oct31 = 304 + 14 Nov = day 20772
        let utc: u64 = 20772 * 86400 + 12 * 3600;
        assert_eq!(utc_secs_to_moscow_string(utc), "2026-11-15 15:00:00 MSK");
    }

    #[test]
    fn midnight_utc_becomes_03_00_msk_same_day() {
        // 2026-01-01 00:00:00 UTC → 2026-01-01 03:00:00 MSK
        // 2026-01-01: 20454 days from epoch
        let utc: u64 = 20454 * 86400;
        let result = utc_secs_to_moscow_string(utc);
        assert_eq!(result, "2026-01-01 03:00:00 MSK");
    }

    #[test]
    fn late_night_utc_crosses_into_next_day_msk() {
        // 2026-11-15 23:00:00 UTC → 2026-11-16 02:00:00 MSK
        let utc: u64 = 20772 * 86400 + 23 * 3600;
        assert_eq!(utc_secs_to_moscow_string(utc), "2026-11-16 02:00:00 MSK");
    }

    #[test]
    fn feb_29_in_leap_year() {
        // 2000-02-29 00:00:00 UTC → 2000-02-29 03:00:00 MSK
        // days from epoch to 2000-02-29:
        //   30 years (1970..1999): 8 leap years (72,76,80,84,88,92,96) = 7+1970?
        //   leap years between 1970 and 1999 inclusive: 1972,1976,1980,1984,1988,1992,1996 = 7
        //   30*365 + 7 = 10950 + 7 = 10957 days through 1999
        //   2000-01: 31 days, 2000-02-29: 29 more → 60 days into 2000
        //   total: 10957 + 59 = 11016 days (0-indexed: day 11016 is 2000-02-29)
        let utc: u64 = 11016 * 86400;
        let result = utc_secs_to_moscow_string(utc);
        assert_eq!(result, "2000-02-29 03:00:00 MSK");
    }

    // ── display ───────────────────────────────────────────────────────────────

    #[test]
    fn display_contains_all_fields() {
        let info = SnapshotInfo {
            build_number: "1600000".to_owned(),
            release_date: "2026-11-15 12:00:00".to_owned(),
            moscow_time: "2026-11-15 15:00:00 MSK".to_owned(),
        };
        let out = info.display();
        assert!(out.contains("1600000"));
        assert!(out.contains("2026-11-15 12:00:00"));
        assert!(out.contains("2026-11-15 15:00:00 MSK"));
    }
}
