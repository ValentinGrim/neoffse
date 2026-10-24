//! From a listed backup to the Slint row.

use neoffse::io::backup::Backup;

use crate::editor::stamp_of;
use crate::home::slot_kind_of;
use crate::{BackupReason, BackupRow};

/// The reason of a backup manifest as the page shows it. The match is exact
/// and case-sensitive: `before-save` and `before-restore` are the two reasons
/// the library writes, every other text is `Other`.
#[must_use]
pub fn reason_of(reason: &str) -> BackupReason {
    match reason {
        "before-save" => BackupReason::BeforeSave,
        "before-restore" => BackupReason::BeforeRestore,
        _ => BackupReason::Other,
    }
}

/// The row of one backup.
///
/// `stamp` is empty when the manifest time does not parse. The slot comes
/// from the first file name alone, by the rule Home uses for a game with no
/// format; a name the rule does not know gives `Other`, and the page then
/// shows `file`. The row holds no folder: the backup's `dir` and the
/// manifest's `source_dir` stay in Rust.
#[must_use]
pub fn backup_row(key: i32, backup: &Backup) -> BackupRow {
    let manifest = &backup.manifest;
    let file = manifest.files.first().map_or("", String::as_str);
    let (slot_kind, slot_number) = slot_kind_of(None, file);
    BackupRow {
        key,
        stamp: stamp_of(&manifest.created).unwrap_or_default().into(),
        reason: reason_of(&manifest.reason),
        slot_kind,
        slot_number,
        file: file.into(),
    }
}

/// The rows of a listing, in its order. The key of a row is its index.
#[must_use]
pub fn backup_rows(backups: &[Backup]) -> Vec<BackupRow> {
    backups
        .iter()
        .enumerate()
        .map(|(index, backup)| backup_row(i32::try_from(index).unwrap_or(i32::MAX), backup))
        .collect()
}
