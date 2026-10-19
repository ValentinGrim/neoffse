//! File handling for NeoFFSE saves.
//!
//! The crate holds four modules: [`error`], the [`IoError`](error::IoError)
//! type that every operation of the crate returns, [`dirs`],
//! [`DataDir`](dirs::DataDir), which names the folder where NeoFFSE keeps its
//! backups, [`atomic`], the verified replacement of a file, and [`backup`],
//! the timestamped copies of a game's save files with their manifest.

pub mod atomic;
pub mod backup;
pub mod dirs;
pub mod error;
