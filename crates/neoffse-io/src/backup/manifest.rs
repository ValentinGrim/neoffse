//! The content of `backup.json` and the instant that names a backup.

use std::collections::BTreeSet;
use std::error::Error;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

/// The name of the manifest inside a backup folder.
pub(crate) const MANIFEST_FILE: &str = "backup.json";

/// The format version this crate writes and reads.
pub const MANIFEST_VERSION: u32 = 1;

/// Largest local time `Moment` holds: 9999-12-31T23:59:59, so the year keeps
/// four digits in the folder name and in `created`.
const MAX_LOCAL_SECONDS: i64 = 253_402_300_799;

/// Largest offset from UTC `Moment` holds: 23:59.
const MAX_OFFSET_SECONDS: u32 = 86_340;

/// An instant with its offset from UTC, at whole-second precision. It names
/// the backup folder and fills `created`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Moment {
    unix_seconds: i64,
    offset_seconds: i32,
}

impl Moment {
    /// The instant `unix_seconds` after 1970-01-01T00:00:00Z, shown with an
    /// offset of `offset_seconds` from UTC.
    ///
    /// Returns `None` unless `offset_seconds` is a whole number of minutes
    /// with an absolute value of at most 86 340 (23:59) and the local time,
    /// `unix_seconds + offset_seconds`, lies in 0 ..= 253 402 300 799
    /// (1970-01-01T00:00:00 to 9999-12-31T23:59:59).
    #[must_use]
    pub fn new(unix_seconds: i64, offset_seconds: i32) -> Option<Self> {
        if offset_seconds % 60 != 0 || offset_seconds.unsigned_abs() > MAX_OFFSET_SECONDS {
            return None;
        }
        let local = unix_seconds.checked_add(i64::from(offset_seconds))?;
        if !(0..=MAX_LOCAL_SECONDS).contains(&local) {
            return None;
        }
        Some(Self { unix_seconds, offset_seconds })
    }

    /// The system clock in UTC (offset 0). A clock that reads before 1970 or
    /// past year 9999 gives 1970-01-01T00:00:00Z.
    #[must_use]
    pub fn now() -> Self {
        let seconds = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .ok()
            .and_then(|elapsed| i64::try_from(elapsed.as_secs()).ok());
        seconds.and_then(|seconds| Self::new(seconds, 0)).unwrap_or(Self {
            unix_seconds: 0,
            offset_seconds: 0,
        })
    }

    /// The local date and time as `(year, month, day, hour, minute, second)`.
    fn civil(self) -> (i64, i64, i64, i64, i64, i64) {
        let local = self.unix_seconds + i64::from(self.offset_seconds);
        let (year, month, day) = civil_from_days(local.div_euclid(86_400));
        let second_of_day = local.rem_euclid(86_400);
        (year, month, day, second_of_day / 3_600, second_of_day % 3_600 / 60, second_of_day % 60)
    }

    /// The backup folder name, `YYYYMMDD-HHMMSS`, in local time.
    pub(crate) fn stamp(self) -> String {
        let (year, month, day, hour, minute, second) = self.civil();
        format!("{year:04}{month:02}{day:02}-{hour:02}{minute:02}{second:02}")
    }

    /// The RFC 3339 text: `Z` for offset 0, else `+HH:MM` or `-HH:MM`.
    pub(crate) fn rfc3339(self) -> String {
        let (year, month, day, hour, minute, second) = self.civil();
        let zone = if self.offset_seconds == 0 {
            "Z".to_owned()
        } else {
            let sign = if self.offset_seconds < 0 { '-' } else { '+' };
            let offset = self.offset_seconds.unsigned_abs();
            format!("{sign}{:02}:{:02}", offset / 3_600, offset % 3_600 / 60)
        };
        format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}{zone}")
    }
}

/// Converts days since 1970-01-01 to a proleptic Gregorian `(year, month,
/// day)`. This is the public domain "civil from days" algorithm of Howard
/// Hinnant: it counts in 400-year eras of 146 097 days, places the year
/// start on 1 March so that the leap day closes the year, then shifts back.
fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let shifted = days + 719_468;
    let era = shifted.div_euclid(146_097);
    let day_of_era = shifted.rem_euclid(146_097);
    let year_of_era = (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let march_month = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * march_month + 2) / 5 + 1;
    let month = if march_month < 10 { march_month + 3 } else { march_month - 9 };
    let year = year_of_era + era * 400 + i64::from(month <= 2);
    (year, month, day)
}

/// The content of `backup.json`.
///
/// The text is JSON with two-space indentation, the keys in field order and
/// one final newline.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct Manifest {
    /// The format version, [`MANIFEST_VERSION`].
    pub version: u32,
    /// The game id.
    pub game: String,
    /// The folder the files came from, as lossy UTF-8.
    pub source_dir: String,
    /// The creation time, RFC 3339: `2026-10-18T22:14:03+02:00` or
    /// `2026-10-18T20:14:03Z`.
    pub created: String,
    /// Plain file names, in request order.
    pub files: Vec<String>,
    /// The kind of save the files hold, for example `slot`.
    pub save_type: String,
    /// Why the backup exists, for example `before-save` or
    /// `before-restore`.
    pub reason: String,
}

type BoxedError = Box<dyn Error + Send + Sync>;

impl Manifest {
    /// Checks the rules a manifest keeps, whether built in memory or read
    /// from disk. The rules keep `dir.join(name)` inside the backup folder.
    pub(crate) fn validate(&self) -> Result<(), BoxedError> {
        if self.version != MANIFEST_VERSION {
            return Err(format!("unsupported manifest version {}", self.version).into());
        }
        if self.files.is_empty() {
            return Err("a backup needs at least one file".into());
        }
        // Windows folds case, so two names that differ by case are one file.
        let mut seen = BTreeSet::new();
        for name in &self.files {
            if !is_plain_file_name(name) {
                return Err(format!("file name {name:?} is not a plain file name").into());
            }
            if !seen.insert(name.to_ascii_lowercase()) {
                return Err(format!("file name {name:?} appears twice").into());
            }
        }
        if self.save_type.is_empty() {
            return Err("saveType is empty".into());
        }
        if self.reason.is_empty() {
            return Err("reason is empty".into());
        }
        Ok(())
    }

    /// Reads a manifest from JSON bytes, then validates it.
    pub(crate) fn parse(bytes: &[u8]) -> Result<Self, BoxedError> {
        let manifest: Self = serde_json::from_slice(bytes)?;
        manifest.validate()?;
        Ok(manifest)
    }
}

/// A name that stays inside its folder on Unix and on Windows and cannot
/// replace the manifest.
fn is_plain_file_name(name: &str) -> bool {
    !name.is_empty() && name != "." && name != ".." && !name.contains(['/', '\\', ':', '\0']) && !name.eq_ignore_ascii_case(MANIFEST_FILE)
}
