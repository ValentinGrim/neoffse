//! Helpers that the `backup_*` test crates share, and the `Snoop` file-ops
//! wrapper. The module sits under `common` and inherits its `dead_code`
//! allowance.

use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io;
use std::path::{Path, PathBuf};

use neoffse_io::atomic::{FileOps, RealFileOps};
use neoffse_io::backup::{Backup, Moment, Request, Restored, create_with, restore_with};
use neoffse_io::dirs::DataDir;
use neoffse_io::error::IoError;

use super::{Log, Probe, Step};

/// 2026-10-18T20:14:03Z.
pub(crate) const T0: i64 = 1_792_354_443;

/// Slot 1 (`pixel-remaster.md#slot-files`). It holds `+` and `=`, legal on
/// both platforms.
pub(crate) const SLOT: &str = "ookrbATYovG3tEOXIH4HqWnsv8TrUlRWzM8AlCmW2mk=";

/// The bestiary file (`pixel-remaster.md#other-files-in-the-folder`).
pub(crate) const BESTIARY: &str = "dp3fS2vqP7GDj8eF72YKqbT7FIAF=e7Shy2CsTITm2E=";

/// A fresh test folder: a data folder that does not exist yet and a source
/// folder that holds `SLOT` and `BESTIARY`.
pub(crate) struct World {
    pub(crate) root: PathBuf,
    pub(crate) data: DataDir,
    pub(crate) source: PathBuf,
}

/// Builds `backup-<name>` under the Cargo target temporary folder, emptied
/// first.
pub(crate) fn world(name: &str) -> World {
    let root = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("backup-{name}"));
    if root.exists() {
        fs::remove_dir_all(&root).unwrap();
    }
    let source = root.join("source");
    fs::create_dir_all(&source).unwrap();
    fs::write(source.join(SLOT), b"slot-one").unwrap();
    fs::write(source.join(BESTIARY), b"bestiary-one").unwrap();
    World {
        data: DataDir::at(root.join("data")),
        root,
        source,
    }
}

/// The instant `unix` in UTC.
pub(crate) fn at(unix: i64) -> Moment {
    Moment::new(unix, 0).unwrap()
}

/// A `slot` request for `before-save`.
pub(crate) fn request<'a>(world: &'a World, game: &'a str, files: &'a [&'a str]) -> Request<'a> {
    Request {
        game,
        source_dir: &world.source,
        files,
        save_type: "slot",
        reason: "before-save",
    }
}

/// Backs up both files at `unix` through `RealFileOps`.
pub(crate) fn make(world: &World, game: &str, unix: i64) -> Backup {
    let files = [SLOT, BESTIARY];
    create_with(&RealFileOps, &world.data, &request(world, game, &files), at(unix)).unwrap()
}

/// The bytes `edit` writes into `SLOT`.
pub(crate) const EDITED_SLOT: &[u8] = b"slot-edited";

/// The bytes `edit` writes into `BESTIARY`.
pub(crate) const EDITED_BESTIARY: &[u8] = b"bestiary-edited";

/// Overwrites both files of the save folder, after a `make` took the old ones.
pub(crate) fn edit(world: &World) {
    fs::write(world.source.join(SLOT), EDITED_SLOT).unwrap();
    fs::write(world.source.join(BESTIARY), EDITED_BESTIARY).unwrap();
}

/// Restores `backup` of game `ff1-pr` into the save folder at `unix`.
pub(crate) fn restore_at(world: &World, backup: &Backup, unix: i64) -> Result<Restored, IoError> {
    restore_with(&RealFileOps, &world.data, "ff1-pr", backup, &world.source, at(unix))
}

/// Every file below `dir`, keyed by the relative path with `/` separators.
/// An empty folder is an entry with a trailing `/` and no bytes.
pub(crate) fn tree(dir: &Path) -> BTreeMap<String, Vec<u8>> {
    let mut entries = BTreeMap::new();
    walk(dir, "", &mut entries);
    entries
}

fn walk(dir: &Path, prefix: &str, entries: &mut BTreeMap<String, Vec<u8>>) {
    let mut empty = true;
    for entry in fs::read_dir(dir).unwrap() {
        let entry = entry.unwrap();
        empty = false;
        let key = format!("{prefix}{}", entry.file_name().to_string_lossy());
        if entry.file_type().unwrap().is_dir() {
            walk(&entry.path(), &format!("{key}/"), entries);
        } else {
            entries.insert(key, fs::read(entry.path()).unwrap());
        }
    }
    if empty && !prefix.is_empty() {
        entries.insert(prefix.to_owned(), Vec::new());
    }
}

/// `<data>/backups/<game>`.
pub(crate) fn game_dir(world: &World, game: &str) -> PathBuf {
    world.data.game_backups_dir(game).unwrap()
}

/// A valid `backup.json` document, in the pretty form `create_with` writes.
pub(crate) fn manifest_text(game: &str, version: u64, files: &[&str]) -> String {
    let files: Vec<String> = files.iter().map(|name| format!("    {}", serde_json::to_string(name).unwrap())).collect();
    format!(
        "{{\n  \"version\": {version},\n  \"game\": {game:?},\n  \"sourceDir\": \"/some/source\",\n  \
         \"created\": \"2026-10-18T22:14:03+02:00\",\n  \"files\": [\n{}\n  ],\n  \"saveType\": \"slot\",\n  \
         \"reason\": \"before-save\"\n}}\n",
        files.join(",\n")
    )
}

/// Creates `<game dir>/<name>` with a file `SLOT` and, for `Some`, a
/// `backup.json` that holds the text. Returns the folder.
pub(crate) fn plant(world: &World, game: &str, name: &str, manifest: Option<&str>) -> PathBuf {
    let dir = game_dir(world, game).join(name);
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join(SLOT), b"planted").unwrap();
    if let Some(text) = manifest {
        fs::write(dir.join("backup.json"), text).unwrap();
    }
    dir
}

/// What `Snoop` does to the bytes of a write.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Corrupt {
    /// Writes the bytes it receives.
    None,
    /// Writes `{}`, which no manifest parser accepts.
    Garbage,
    /// Writes the bytes with `before-save` changed to `before-sav3`: the
    /// document still parses and differs in one field.
    Field,
}

/// A `FileOps` that wraps a `Probe` and records the content of the folder at
/// each `create_temp`. Its `Corrupt` mode changes the bytes of a write.
#[derive(Debug)]
pub(crate) struct Snoop {
    probe: Probe,
    log: Log,
    corrupt: Corrupt,
}

impl Snoop {
    /// A snoop that fails `fail_at`, with the log it writes to. With
    /// `corrupt_write` it writes `Corrupt::Garbage`.
    pub(crate) fn new(fail_at: &[Step], corrupt_write: bool) -> (Self, Log) {
        Self::corrupting(fail_at, if corrupt_write { Corrupt::Garbage } else { Corrupt::None })
    }

    /// A snoop that fails `fail_at` and changes each write as `corrupt` says.
    pub(crate) fn corrupting(fail_at: &[Step], corrupt: Corrupt) -> (Self, Log) {
        let (probe, log) = Probe::new(fail_at);
        (
            Self {
                probe,
                log: Log::clone(&log),
                corrupt,
            },
            log,
        )
    }
}

impl FileOps for Snoop {
    fn create_temp(&self, dir: &Path) -> io::Result<(File, PathBuf)> {
        let mut found: Vec<String> = fs::read_dir(dir)?
            .map(|entry| entry.map(|entry| entry.file_name().to_string_lossy().into_owned()))
            .collect::<io::Result<_>>()?;
        found.sort();
        self.log.borrow_mut().push(format!("listing|{}", found.join(",")));
        self.probe.create_temp(dir)
    }

    fn write_all(&self, file: &mut File, bytes: &[u8]) -> io::Result<()> {
        match self.corrupt {
            Corrupt::None => self.probe.write_all(file, bytes),
            Corrupt::Garbage => self.probe.write_all(file, b"{}\n"),
            Corrupt::Field => {
                let text = String::from_utf8_lossy(bytes).replace("before-save", "before-sav3");
                self.probe.write_all(file, text.as_bytes())
            }
        }
    }

    fn copy_permissions(&self, from: &Path, file: &File) -> io::Result<()> {
        self.probe.copy_permissions(from, file)
    }

    fn sync_file(&self, file: &File) -> io::Result<()> {
        self.probe.sync_file(file)
    }

    fn rename(&self, from: &Path, to: &Path) -> io::Result<()> {
        self.probe.rename(from, to)
    }

    fn remove_file(&self, path: &Path) -> io::Result<()> {
        self.probe.remove_file(path)
    }

    fn sync_dir(&self, dir: &Path) -> io::Result<()> {
        self.probe.sync_dir(dir)
    }
}

/// The names that the first `listing` entry of the log holds.
pub(crate) fn listing(log: &Log) -> Vec<String> {
    let log = log.borrow();
    let entry = log
        .iter()
        .find_map(|entry| entry.strip_prefix("listing|"))
        .unwrap_or_else(|| panic!("no `listing` entry in {log:?}"));
    entry.split(',').filter(|name| !name.is_empty()).map(str::to_owned).collect()
}
