//! The restore of a backup and the listing of backups, on the registry.

use std::path::Path;

use neoffse_core::Game;
use neoffse_io::atomic::{FileOps, RealFileOps};
use neoffse_io::backup::{self, Backup, Listing, Moment, Restored};
use neoffse_io::dirs::DataDir;
use neoffse_io::process;

use crate::error::Error;
use crate::executables::refuse_if_running;
use crate::registry::Registry;
use crate::session::Running;

impl Registry {
    /// Lists the backups of `game`, newest first.
    ///
    /// A game with no backup folder gives an empty listing. A backup folder
    /// whose manifest is missing, invalid or names another game lands in
    /// [`Listing::unreadable`].
    ///
    /// # Errors
    ///
    /// [`Error::Io`] when the game's backup folder exists and cannot be
    /// listed.
    #[allow(clippy::unused_self, reason = "the app reaches every save operation through the registry")]
    pub fn backups(&self, data: &DataDir, game: Game) -> Result<Listing, Error> {
        backup::list(data, game.id()).map_err(|source| Error::Io { source })
    }

    /// Writes the files of `backup` over the files of `target_dir`, after a
    /// backup of the files they replace.
    ///
    /// The restore needs no [`Session`](crate::Session): the main case is a
    /// file that no longer opens. It copies the backup's bytes as they are
    /// and opens no format, so a backup that holds a broken file restores that
    /// broken file. With [`Running::Refuse`], it first scans for the game's
    /// executable and stops when it finds one. The backup's files are never
    /// read in that case, and nothing is written.
    ///
    /// The app reopens the file with [`Registry::open`] afterwards. A
    /// `Session` already open on a restored file is stale: its next `save`
    /// returns [`Error::ChangedOnDisk`].
    ///
    /// To undo a restore, restore [`Restored::safety`], the `before-restore`
    /// backup of the files the restore replaced. Files in
    /// [`Restored::created`] were absent from `target_dir` and stay after that
    /// undo.
    ///
    /// `target_dir` comes from discovery or from
    /// [`Session::dir`](crate::Session::dir), never from `Manifest::source_dir`
    /// without the caller's choice.
    ///
    /// # Errors
    ///
    /// - [`Error::GameRunning`] with [`Running::Refuse`] when the process scan
    ///   finds the backup's game.
    /// - [`Error::Io`] for every other failure, including
    ///   `RestoreFailed` when the write stops after some files.
    #[allow(clippy::unused_self, reason = "the app reaches every save operation through the registry")]
    pub fn restore(&self, data: &DataDir, backup: &Backup, target_dir: &Path, running: Running) -> Result<Restored, Error> {
        self.restore_with(&RealFileOps, process::running, Moment::now(), data, backup, target_dir, running)
    }

    /// The steps of [`Registry::restore`] with a chosen [`FileOps`], process
    /// probe and instant. The app calls `restore`; `restore_with` exists for
    /// failure-injection tests. `probe` receives the executable names of the
    /// backup's game and returns those it found running.
    ///
    /// # Errors
    ///
    /// Same as [`Registry::restore`].
    #[allow(clippy::unused_self, reason = "the app reaches every save operation through the registry")]
    #[allow(clippy::too_many_arguments, reason = "mirrors Session::save_with plus the backup and the target folder")]
    pub fn restore_with<O, P>(
        &self,
        ops: &O,
        probe: P,
        now: Moment,
        data: &DataDir,
        backup: &Backup,
        target_dir: &Path,
        running: Running,
    ) -> Result<Restored, Error>
    where
        O: FileOps + ?Sized,
        P: Fn(&[&'static str]) -> Vec<&'static str>,
    {
        let game = backup.manifest.game.as_str();
        refuse_if_running(game, running, probe)?;
        backup::restore_with(ops, data, game, backup, target_dir, now).map_err(|source| Error::Io { source })
    }
}
