//! File handling for NeoFFSE saves.
//!
//! The crate holds seven modules: [`error`], the [`IoError`](error::IoError)
//! type that every operation of the crate returns, [`dirs`],
//! [`DataDir`](dirs::DataDir), which names the folder where NeoFFSE keeps its
//! backups, [`atomic`], the verified replacement of a file, [`backup`], the
//! timestamped copies of a game's save files with their manifest,
//! [`discovery`], the search for the save folders of Steam games, [`files`],
//! the listing and reading of a save folder, and [`process`], the check for a
//! running game.

pub mod atomic;
pub mod backup;
pub mod dirs;
pub mod discovery;
pub mod error;
pub mod files;
pub mod process;
