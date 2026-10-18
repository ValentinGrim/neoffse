//! Public facade of NeoFFSE: the save editing library, without any UI.
//! It re-exports the internal crates so a caller depends on `neoffse`
//! alone, and holds what joins them: the format registry, the edit
//! history and the save pipeline. `neoffse-app` is one client; any other
//! project can be another.
