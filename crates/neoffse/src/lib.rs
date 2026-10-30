//! Public facade of NeoFFSE: the save editing library, without any UI.
//! It is the crate that other projects and `neoffse-app` depend on.
//!
//! The facade re-exports the internal crates as modules: [`core`] (the model
//! and the format traits), [`data`] (the ID tables), [`io`] (atomic writes,
//! backups, discovery) and [`pr`] (the Pixel Remaster formats). It holds
//! what joins them today:
//!
//! - [`Registry`]: the formats NeoFFSE ships. It claims the files of a
//!   folder ([`Registry::scan`]) and opens one ([`Registry::open`]).
//! - [`Session`]: one opened save with its edit history. It edits in the
//!   default or the expert mode, undoes and redoes, tells whether the
//!   document differs from the files read, discards the edits by
//!   reopening the bytes held in memory, and saves ([`Session::save`]): a
//!   running-game check, a newer-on-disk check, the format's encoder, a
//!   backup and the verified atomic write of [`io`].
//! - [`changes`]: the net difference between two views. [`Session::changes`]
//!   lists what differs from the clean state, field by field, for a review
//!   before the save.
//! - [`Registry::backups`] and [`Registry::restore`]: the registry also lists
//!   the backups of a game and restores one, with no session and no format:
//!   the main case is a file that no longer opens.
//! - [`Error`]: what the registry and the session report.
//!
//! `pub use neoffse_core as core` shadows the `core` prelude crate at this
//! crate's root: inside the facade, write `::core` for the standard library.

pub use neoffse_core as core;
pub use neoffse_data as data;
pub use neoffse_format_pr as pr;
pub use neoffse_io as io;

pub mod changes;
mod error;
mod executables;
mod registry;
mod restore;
mod session;

pub use changes::{Change, ChangeKind};
pub use error::Error;
pub use registry::{Claim, Found, Registry, Scan};
pub use session::{Running, Saved, Session};
