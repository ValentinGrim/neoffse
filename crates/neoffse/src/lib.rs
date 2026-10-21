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
//! - [`Error`]: what the registry reports.
//!
//! The edit history, the save pipeline that links a format's encoder to the
//! verified write of [`io`], and the restore of a backup come next.
//!
//! `pub use neoffse_core as core` shadows the `core` prelude crate at this
//! crate's root: inside the facade, write `::core` for the standard library.

pub use neoffse_core as core;
pub use neoffse_data as data;
pub use neoffse_format_pr as pr;
pub use neoffse_io as io;

mod error;
mod registry;

pub use error::Error;
pub use registry::{Claim, Found, OpenSave, Registry, Scan};
