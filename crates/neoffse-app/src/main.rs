//! NeoFFSE desktop editor binary. All logic lives in the `neoffse_app` library.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![allow(
    clippy::multiple_crate_versions,
    reason = "Slint's tree (winit, wayland, objc2, windows-sys, rustix, hashbrown, thiserror, tiny-skia and others) resolves several crates at more than one version; the workspace cannot unify them"
)]

fn main() -> Result<(), slint::PlatformError> {
    neoffse_app::run()
}
