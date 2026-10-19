//! File handling for NeoFFSE saves.
//!
//! The crate holds two modules: [`error`], the [`IoError`](error::IoError)
//! type that every operation of the crate returns, and [`dirs`],
//! [`DataDir`](dirs::DataDir), which names the folder where NeoFFSE keeps its
//! backups.

pub mod dirs;
pub mod error;
