//! Key and pointer events sent through `Window::dispatch_event` on the
//! testing backend, and the editor navigation state.

use i_slint_backend_testing::{AccessibleRole, ElementHandle};
use slint::platform::{Key, PointerEventButton, WindowEvent};
use slint::{ComponentHandle, LogicalPosition, SharedString};

use neoffse_app::EditorCommand;

use super::Ui;

impl Ui {
    /// Presses and releases the key that types `text`.
    pub(crate) fn press(&self, text: impl Into<SharedString>) {
        let text = text.into();
        self.window.window().dispatch_event(WindowEvent::KeyPressed { text: text.clone() });
        self.window.window().dispatch_event(WindowEvent::KeyReleased { text });
    }

    /// Presses and releases a named key.
    pub(crate) fn key(&self, key: Key) {
        self.press(SharedString::from(key));
    }

    /// A left click at `x`, `y` in logical pixels.
    pub(crate) fn click_at(&self, x: f32, y: f32) {
        let position = LogicalPosition::new(x, y);
        self.window.window().dispatch_event(WindowEvent::PointerPressed {
            position,
            button: PointerEventButton::Left,
        });
        self.window.window().dispatch_event(WindowEvent::PointerReleased {
            position,
            button: PointerEventButton::Left,
        });
    }

    /// Puts the editor on `command` and `character` through the bridge.
    pub(crate) fn navigate(&self, command: EditorCommand, character: i32) {
        self.bridge().set_command(command);
        self.bridge().set_character(character);
    }

    /// Whether the editor shows the Party cards.
    pub(crate) fn on_cards(&self) -> bool {
        self.bridge().get_command() == EditorCommand::Party && self.bridge().get_character() == -1
    }

    /// Makes the session dirty with one gil edit.
    pub(crate) fn make_dirty(&self) {
        self.commit(self.gil().key, "500");
        assert_eq!(self.pending(), 1);
    }

    /// The element with `role` whose accessible label is `label`.
    pub(crate) fn element(&self, label: &str, role: AccessibleRole) -> ElementHandle {
        ElementHandle::find_by_accessible_label(&self.window, label)
            .find(|element| element.accessible_role() == Some(role))
            .unwrap_or_else(|| panic!("no {role:?} labelled {label}"))
    }

    /// Whether an element with `role` and `label` exists now.
    pub(crate) fn has_element(&self, label: &str, role: AccessibleRole) -> bool {
        ElementHandle::find_by_accessible_label(&self.window, label).any(|element| element.accessible_role() == Some(role))
    }

    /// The text box labelled `label`.
    pub(crate) fn input(&self, label: &str) -> ElementHandle {
        self.element(label, AccessibleRole::TextInput)
    }

    /// The text of the box labelled `label`.
    pub(crate) fn input_text(&self, label: &str) -> String {
        self.input(label).accessible_value().expect("a text value").to_string()
    }

    /// Clicks the text box labelled `label`, which gives it the focus.
    pub(crate) fn focus_input(&self, label: &str) {
        self.input(label).mock_single_click(PointerEventButton::Left);
    }

    /// Types `text`, one key per character.
    pub(crate) fn type_text(&self, text: &str) {
        for character in text.chars() {
            self.press(character.to_string());
        }
    }

    /// Replaces the text of the focused box: End, then Backspace once per character.
    pub(crate) fn replace_text(&self, label: &str, text: &str) {
        self.key(Key::End);
        for _ in self.input_text(label).chars() {
            self.key(Key::Backspace);
        }
        self.type_text(text);
    }
}
