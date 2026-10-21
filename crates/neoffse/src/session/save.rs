//! The save pipeline of a session.

use std::error::Error as StdError;
use std::fs;
use std::path::Path;

use neoffse_core::{FormatError, Game, SaveFiles};
use neoffse_format_pr::games::GAMES;
use neoffse_io::atomic::{self, FileOps, RealFileOps};
use neoffse_io::backup::{self, Backup, Moment, Request};
use neoffse_io::dirs::DataDir;
use neoffse_io::{files, process};

use super::Session;
use crate::error::Error;

/// Whether [`Session::save`] stops when the game's process is found.
///
/// The process check is best effort (`docs/architecture.md#save-safety`,
/// Process matching): a process the system hides stays invisible, and an
/// empty result proves nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Running {
    /// Stop with [`Error::GameRunning`] when the game's process is found.
    Refuse,
    /// The user chose to continue: no process scan.
    Allow,
}

/// What a save did.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Saved {
    /// The backup taken before the write. `None` when the session was clean
    /// and nothing was written.
    pub backup: Option<Backup>,
}

impl Session {
    /// Writes the edited document over the file it was read from, after a
    /// backup, and makes the written bytes the new clean point.
    ///
    /// The steps run in this order, and the first failure stops the save:
    ///
    /// 1. A clean session saves nothing and returns `Saved { backup: None }`.
    ///    No process scan, no read, no backup, no write.
    /// 2. With [`Running::Refuse`], the process scan for the game's
    ///    executable.
    /// 3. A re-read of every file the session read: bytes that differ from
    ///    [`Session::original`] mean another program changed the file, and the
    ///    save stops.
    /// 4. The format's encoder.
    /// 5. A backup of the files as they are on disk.
    /// 6. A verified atomic write of each file: the temporary file is read
    ///    back and the format compares it with the document before the rename.
    /// 7. The written bytes replace [`Session::original`]. The history stays,
    ///    so undo and redo still work, and the dirty count restarts from the
    ///    new clean point.
    ///
    /// The save writes the files it read and no other: a file the encoder
    /// returns under another name is not written. A format with several files
    /// writes them one by one, so a failure after the first rename leaves the
    /// earlier files written. The backup holds the files as they were. The
    /// Pixel Remaster formats have one file.
    ///
    /// # Errors
    ///
    /// - [`Error::GameRunning`] with [`Running::Refuse`] when the process
    ///   scan finds the game.
    /// - [`Error::ChangedOnDisk`] when a file differs from what the session
    ///   read.
    /// - [`Error::Format`] when the encoder fails or omits a file the
    ///   session read.
    /// - [`Error::Io`] for a read, backup or write failure, including
    ///   `VerificationFailed` when the written file does not match the
    ///   document.
    ///
    /// On any error the document, the history, the clean point and
    /// [`Session::original`] stay as they were.
    pub fn save(&mut self, data: &DataDir, running: Running) -> Result<Saved, Error> {
        self.save_with(&RealFileOps, process::running, Moment::now(), data, running)
    }

    /// The steps of [`Session::save`] with a chosen [`FileOps`], process probe
    /// and instant. The app calls `save`; `save_with` exists for
    /// failure-injection tests. `probe` receives the executable names of the
    /// session's game and returns those it found running.
    ///
    /// # Errors
    ///
    /// Same as [`Session::save`].
    pub fn save_with<O, P>(&mut self, ops: &O, probe: P, now: Moment, data: &DataDir, running: Running) -> Result<Saved, Error>
    where
        O: FileOps + ?Sized,
        P: Fn(&[&'static str]) -> Vec<&'static str>,
    {
        if !self.is_dirty() {
            return Ok(Saved { backup: None });
        }
        if running == Running::Refuse {
            let executables = executables_of(self.game);
            if !executables.is_empty() {
                let found = probe(&executables);
                if !found.is_empty() {
                    return Err(Error::GameRunning { executables: found });
                }
            }
        }
        self.refuse_if_changed()?;

        let encoded = self.document.encode().map_err(|source| Error::Format { source })?;
        let names: Vec<&str> = self.original.iter().map(|(name, _)| name).collect();
        let mut written = SaveFiles::new();
        for name in &names {
            let bytes = encoded.get(name).ok_or_else(|| Error::Format {
                source: FormatError::MissingFile { name: (*name).to_owned() },
            })?;
            written.push(*name, bytes.to_vec()).map_err(|source| Error::Format { source })?;
        }

        let request = Request {
            game: self.game.id(),
            source_dir: &self.dir,
            files: &names,
            save_type: "slot",
            reason: "before-save",
        };
        let backup = backup::create_with(ops, data, &request, now).map_err(|source| Error::Io { source })?;

        for name in &names {
            let Some(bytes) = written.get(name) else { continue };
            let hook = |temp: &Path| -> Result<(), Box<dyn StdError + Send + Sync>> {
                let on_disk = fs::read(temp)?;
                let files = replace_file(&encoded, name, on_disk)?;
                self.document.verify(&files)?;
                Ok(())
            };
            atomic::write_with(ops, &self.dir.join(name), bytes, hook).map_err(|source| Error::Io { source })?;
        }

        self.original = written;
        self.base = self.undo.len();
        self.extra = 0;
        Ok(Saved { backup: Some(backup) })
    }

    fn refuse_if_changed(&self) -> Result<(), Error> {
        for (name, bytes) in self.original.iter() {
            let on_disk = files::read(&self.dir, name).map_err(|source| Error::Io { source })?;
            if on_disk != bytes {
                return Err(Error::ChangedOnDisk { name: name.to_owned() });
            }
        }
        Ok(())
    }
}

/// The executable names of `game`: one for a Pixel Remaster game, none for a
/// game with no `GAMES` row.
fn executables_of(game: Game) -> Vec<&'static str> {
    GAMES.iter().filter(|row| row.id == game.id()).map(|row| row.executable).collect()
}

/// `files` with the bytes of `name` replaced by `bytes`.
fn replace_file(files: &SaveFiles, name: &str, bytes: Vec<u8>) -> Result<SaveFiles, FormatError> {
    let mut replaced = SaveFiles::new();
    let mut bytes = Some(bytes);
    for (present, original) in files.iter() {
        let content = if present == name {
            bytes.take().unwrap_or_default()
        } else {
            original.to_vec()
        };
        replaced.push(present, content)?;
    }
    Ok(replaced)
}
