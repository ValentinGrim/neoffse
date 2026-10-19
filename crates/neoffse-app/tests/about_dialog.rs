//! Offscreen tests for the About dialog wiring, run on Slint's testing
//! backend. No test opens a window or starts an event loop.

use neoffse_app::{MainWindow, connect_about, new_about_dialog};
use slint::ComponentHandle;

/// Installs the testing platform for the calling thread. Must run before
/// any component exists.
fn init_testing_backend() {
    i_slint_backend_testing::init_no_event_loop();
}

#[test]
fn main_window_instantiates() -> Result<(), slint::PlatformError> {
    init_testing_backend();
    let main_window = MainWindow::new()?;
    assert!(!main_window.window().is_visible());
    Ok(())
}

#[test]
fn about_dialog_starts_hidden() -> Result<(), slint::PlatformError> {
    init_testing_backend();
    let main_window = MainWindow::new()?;
    let about = new_about_dialog()?;
    connect_about(&main_window, &about);
    assert!(!about.window().is_visible());
    Ok(())
}

#[test]
fn about_dialog_shows_cargo_version() -> Result<(), slint::PlatformError> {
    init_testing_backend();
    let about = new_about_dialog()?;
    let version = about.get_app_version();
    assert_eq!(version, env!("CARGO_PKG_VERSION"));
    assert_eq!(version.split('.').count(), 3);
    assert!(version.split('.').all(|part| !part.is_empty()));
    Ok(())
}

#[test]
fn about_dialog_shows_license_expression() -> Result<(), slint::PlatformError> {
    init_testing_backend();
    let about = new_about_dialog()?;
    assert_eq!(about.get_license(), "MIT OR Apache-2.0");
    Ok(())
}

#[test]
fn about_request_shows_dialog() -> Result<(), slint::PlatformError> {
    init_testing_backend();
    let main_window = MainWindow::new()?;
    let about = new_about_dialog()?;
    connect_about(&main_window, &about);
    // Keep only a weak handle: the glue must own the dialog by itself.
    let weak = about.as_weak();
    drop(about);
    main_window.invoke_about_requested();
    assert!(weak.upgrade().is_some_and(|dialog| dialog.window().is_visible()));
    Ok(())
}

#[test]
fn dismiss_hides_dialog() -> Result<(), slint::PlatformError> {
    init_testing_backend();
    let main_window = MainWindow::new()?;
    let about = new_about_dialog()?;
    connect_about(&main_window, &about);
    main_window.invoke_about_requested();
    assert!(about.window().is_visible());
    about.invoke_dismiss();
    assert!(!about.window().is_visible());
    Ok(())
}

#[test]
fn about_dialog_reopens_after_dismiss() -> Result<(), slint::PlatformError> {
    init_testing_backend();
    let main_window = MainWindow::new()?;
    let about = new_about_dialog()?;
    connect_about(&main_window, &about);
    // Keep only a weak handle: the glue must own the dialog by itself.
    let weak = about.as_weak();
    drop(about);
    main_window.invoke_about_requested();
    let Some(dialog) = weak.upgrade() else {
        return Err(slint::PlatformError::Other("the dialog did not outlive the caller's handle".into()));
    };
    dialog.invoke_dismiss();
    drop(dialog);
    main_window.invoke_about_requested();
    assert!(weak.upgrade().is_some_and(|dialog| dialog.window().is_visible()));
    Ok(())
}

#[test]
fn dismiss_on_hidden_dialog_is_harmless() -> Result<(), slint::PlatformError> {
    init_testing_backend();
    let main_window = MainWindow::new()?;
    let about = new_about_dialog()?;
    connect_about(&main_window, &about);
    about.invoke_dismiss();
    assert!(!about.window().is_visible());
    Ok(())
}

#[test]
fn closing_main_window_hides_about_dialog() -> Result<(), slint::PlatformError> {
    init_testing_backend();
    let main_window = MainWindow::new()?;
    let about = new_about_dialog()?;
    connect_about(&main_window, &about);
    main_window.show()?;
    main_window.invoke_about_requested();
    assert!(main_window.window().is_visible());
    assert!(about.window().is_visible());
    main_window.window().dispatch_event(slint::platform::WindowEvent::CloseRequested);
    assert!(!main_window.window().is_visible());
    assert!(!about.window().is_visible());
    Ok(())
}
