//! File handling for NeoFFSE saves.
//!
//! The crate holds three modules: [`error`], the [`IoError`](error::IoError)
//! type that every operation of the crate returns, [`dirs`],
//! [`DataDir`](dirs::DataDir), which names the folder where NeoFFSE keeps its
//! backups, and [`atomic`], the verified replacement of a file.

pub mod atomic;
pub mod dirs;
pub mod error;
