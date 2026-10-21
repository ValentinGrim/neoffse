//! Public facade of NeoFFSE: the save editing library, without any UI.
//! It is the crate that other projects and `neoffse-app` depend on.
//! The crate exports nothing yet. It will re-export the internal crates and
//! hold what joins them: the format registry, the edit history and the save
//! pipeline that links a format's encoder to the verified write of `neoffse-io`.
