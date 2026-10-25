//! Geometry read from the accessibility tree of the testing backend: the
//! bounds of an element and of the party cards in view.

use i_slint_backend_testing::{AccessibleRole, ElementHandle};
use slint::{ComponentHandle, LogicalSize};

use super::Ui;

/// The tolerance of every bounds comparison, in logical pixels.
pub(crate) const SLACK: f32 = 0.5;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Rect {
    pub(crate) x: f32,
    pub(crate) y: f32,
    pub(crate) w: f32,
    pub(crate) h: f32,
}

impl Rect {
    pub(crate) fn of(element: &ElementHandle) -> Self {
        let (position, size) = (element.absolute_position(), element.size());
        Self {
            x: position.x,
            y: position.y,
            w: size.width,
            h: size.height,
        }
    }

    pub(crate) fn right(&self) -> f32 {
        self.x + self.w
    }

    pub(crate) fn bottom(&self) -> f32 {
        self.y + self.h
    }

    pub(crate) fn holds(&self, inner: &Self) -> bool {
        inner.x >= self.x - SLACK && inner.y >= self.y - SLACK && inner.right() <= self.right() + SLACK && inner.bottom() <= self.bottom() + SLACK
    }
}

/// A card the testing backend can see: its bounds, every text inside it with
/// its label, its portrait frame and its sprite when it has one. A card clipped away by the
/// scroll panel is absent.
pub(crate) struct Card {
    pub(crate) name: String,
    pub(crate) bounds: Rect,
    pub(crate) texts: Vec<(String, Rect)>,
    pub(crate) frame: Option<Rect>,
    pub(crate) sprite: Option<Rect>,
}

impl Card {
    /// The first text labelled `label`.
    pub(crate) fn text(&self, label: &str) -> Rect {
        self.texts
            .iter()
            .find(|(text, _)| text == label)
            .unwrap_or_else(|| panic!("{} has no text labelled {label}", self.name))
            .1
    }

    /// The card lists all its texts: the panel did not cut it.
    pub(crate) fn whole(&self) -> bool {
        self.texts.len() >= 10
    }
}

pub(crate) fn resize(ui: &Ui, width: f32, height: f32) {
    ui.window.window().set_size(LogicalSize::new(width, height));
}

/// The cards in view, top to bottom.
pub(crate) fn visible_cards(ui: &Ui) -> Vec<Card> {
    ElementHandle::find_by_element_type_name(&ui.window, "CharacterRow")
        .map(|row| {
            let button = row
                .query_descendants()
                .match_accessible_role(AccessibleRole::Button)
                .find_first()
                .expect("a focus scope");
            let texts = row
                .query_descendants()
                .match_accessible_role(AccessibleRole::Text)
                .find_all()
                .iter()
                .map(|text| (text.accessible_label().map(|label| label.to_string()).unwrap_or_default(), Rect::of(text)))
                .collect();
            let frame = row
                .query_descendants()
                .match_type_name("PortraitFrame")
                .find_first()
                .map(|frame| Rect::of(&frame));
            let sprite = row
                .query_descendants()
                .match_id("PortraitFrame::sprite")
                .find_first()
                .map(|sprite| Rect::of(&sprite));
            Card {
                name: button.accessible_label().expect("a name").to_string(),
                bounds: Rect::of(&row),
                texts,
                frame,
                sprite,
            }
        })
        .collect()
}
