//! Offscreen tests for the About dialog wiring, run on Slint's testing
//! backend. No test opens a desktop window or starts an event loop.

use i_slint_backend_testing::{AccessibleRole, ElementHandle};
use neoffse_app::{AboutDialog, FontCredit, MainWindow, connect_about, font_credit, new_about_dialog, ui_font_credit};
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

/// Whether the dialog draws a text element reading `sentence`.
fn shows(about: &AboutDialog, sentence: &str) -> bool {
    ElementHandle::find_by_accessible_label(about, sentence).any(|element| element.accessible_role() == Some(AccessibleRole::Text))
}

fn credit(family: &str, author: &str, license: &str) -> FontCredit {
    FontCredit {
        family: family.into(),
        author: author.into(),
        license: license.into(),
    }
}

#[test]
fn about_dialog_shows_both_font_credits() -> Result<(), slint::PlatformError> {
    init_testing_backend();
    let about = new_about_dialog()?;
    assert!(shows(&about, "Cornelia Sans by EliDirkx98, CC BY-SA 3.0"));
    assert!(shows(&about, "Barlow by The Barlow Project Authors, SIL OFL 1.1"));
    let pixel = font_credit();
    assert_eq!(
        (pixel.family.as_str(), pixel.author.as_str(), pixel.license.as_str()),
        ("Cornelia Sans", "EliDirkx98", "CC BY-SA 3.0")
    );
    let app = ui_font_credit();
    assert_eq!(
        (app.family.as_str(), app.author.as_str(), app.license.as_str()),
        ("Barlow", "The Barlow Project Authors", "SIL OFL 1.1")
    );
    Ok(())
}

#[test]
fn the_credit_sentence_is_written_in_markup() -> Result<(), slint::PlatformError> {
    init_testing_backend();
    let about = new_about_dialog()?;
    about.set_font_credit(credit("Aa", "Bb", "Cc"));
    assert!(shows(&about, "Aa by Bb, Cc"));
    about.set_ui_font_credit(credit("Dd", "Ee", "Ff"));
    assert!(shows(&about, "Dd by Ee, Ff"));
    assert!(shows(&about, "Aa by Bb, Cc"), "the first credit stays");
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
