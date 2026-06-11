//! Time-stamping for a run. A `RunStamp` carries two distinct things that were once
//! conflated: an opaque `id` (the evidence-bundle directory name — format private,
//! never parsed) and `at`, an ISO-8601 **UTC** timestamp for humans and for
//! comparisons. The registry stores both; `ls` shows `at`.

use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};

pub struct RunStamp {
    pub id: String,
    pub at: String,
}

impl RunStamp {
    /// Stamp the current wall-clock moment. Called only at the binary's edge; the core
    /// commands take a `&RunStamp` so they stay deterministic under test.
    pub fn now() -> Result<RunStamp> {
        let since = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .context("system clock is before the Unix epoch")?;
        Ok(RunStamp {
            // Sub-second precision keeps two runs in the same second from colliding.
            id: format!("{:010}_{:09}", since.as_secs(), since.subsec_nanos()),
            at: iso8601_utc(since.as_secs()),
        })
    }
}

/// Format Unix seconds as an ISO-8601 UTC timestamp, e.g. `2026-06-07T14:30:03Z`.
/// Uses Howard Hinnant's `civil_from_days` so it needs no date dependency.
pub fn iso8601_utc(unix_secs: u64) -> String {
    let days = (unix_secs / 86_400) as i64;
    let secs_of_day = (unix_secs % 86_400) as i64;
    let (hour, minute, second) = (
        secs_of_day / 3600,
        (secs_of_day % 3600) / 60,
        secs_of_day % 60,
    );

    // civil_from_days: days is the count since 1970-01-01.
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097; // [0, 146096]
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365; // [0, 399]
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // [0, 365]
    let mp = (5 * doy + 2) / 153; // [0, 11]
    let day = doy - (153 * mp + 2) / 5 + 1; // [1, 31]
    let month = if mp < 10 { mp + 3 } else { mp - 9 }; // [1, 12]
    let year = yoe + era * 400 + i64::from(month <= 2);

    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}Z")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_format_known_unix_seconds_as_iso8601_utc() {
        assert_eq!(iso8601_utc(0), "1970-01-01T00:00:00Z");
        assert_eq!(iso8601_utc(86_400), "1970-01-02T00:00:00Z");
        // A well-known epoch.
        assert_eq!(iso8601_utc(1_700_000_000), "2023-11-14T22:13:20Z");
        // A leap day.
        assert_eq!(iso8601_utc(951_782_400), "2000-02-29T00:00:00Z");
    }
}
