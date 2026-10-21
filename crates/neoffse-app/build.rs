//! Compiles `ui/app.slint` into Rust for `slint::include_modules!`.

/// Pins the `fluent` style so the look does not depend on a developer's Qt
/// install or an inherited `SLINT_STYLE`.
///
/// Also asks the `slint!` macro for debug info, which the testing backend's
/// `ElementHandle` needs to read accessible roles and labels in the control
/// tests. The library itself uses no `slint!`, so the setting costs nothing
/// there. The compiled `ui/app.slint` carries the same info outside release
/// builds, so the editor tests can find the text boxes and buttons of the
/// real window.
fn main() -> Result<(), slint_build::CompileError> {
    println!("cargo:rustc-env=SLINT_EMIT_DEBUG_INFO=1");
    let release = std::env::var("PROFILE").is_ok_and(|profile| profile == "release");
    let config = slint_build::CompilerConfiguration::new().with_style("fluent".into()).with_debug_info(!release);
    slint_build::compile_with_config("ui/app.slint", config)
}
