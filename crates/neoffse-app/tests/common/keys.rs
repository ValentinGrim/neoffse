//! Key and pointer events sent through `Window::dispatch_event` on the
//! testing backend, and the editor navigation state.

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
}
