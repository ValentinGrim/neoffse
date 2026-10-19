//! Compiles `ui/app.slint` into Rust for `slint::include_modules!`.

/// Pins the `fluent` style so the look does not depend on a developer's Qt
/// install or an inherited `SLINT_STYLE`.
fn main() -> Result<(), slint_build::CompileError> {
    slint_build::compile_with_config("ui/app.slint", slint_build::CompilerConfiguration::new().with_style("fluent".into()))
}
