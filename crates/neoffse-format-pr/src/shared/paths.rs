//! Document paths of the three collections of a slot: the characters and the
//! two item lists, each an array `target` inside a string document.

use neoffse_core::{FieldId, Key};

use super::names;
use crate::document::Segment;

/// A path into the document; every key name is a `'static` literal.
pub(crate) type Path = Vec<Segment<'static>>;

/// The two lists of `userData` that hold `{contentId, count}` rows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum List {
    /// `normalOwnedItemList`: items and equipment.
    Inventory,
    /// `importantOwendItemList`: key items.
    KeyItems,
}

impl List {
    /// The document key of the list (the game's spelling).
    fn document_key(self) -> &'static str {
        match self {
            List::Inventory => "normalOwnedItemList",
            List::KeyItems => "importantOwendItemList",
        }
    }

    /// The section name of the list's rows.
    pub(crate) fn section(self) -> &'static str {
        match self {
            List::Inventory => names::INVENTORY,
            List::KeyItems => names::KEY_ITEMS,
        }
    }

    /// Path to the `target` array of the list.
    pub(crate) fn target(self) -> Path {
        vec![
            Segment::Key("userData"),
            Segment::Doc,
            Segment::Key(self.document_key()),
            Segment::Doc,
            Segment::Key("target"),
        ]
    }
}

/// A collection of the model, named by its section.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Collection {
    /// `ownedCharacterList`.
    Character,
    /// One of the item lists.
    Items(List),
}

impl Collection {
    /// The collection that `field` names exactly, if any.
    pub(crate) fn of(field: &FieldId) -> Option<Self> {
        match field.keys() {
            [Key::Name(names::CHARACTER)] => Some(Self::Character),
            [Key::Name(names::INVENTORY)] => Some(Self::Items(List::Inventory)),
            [Key::Name(names::KEY_ITEMS)] => Some(Self::Items(List::KeyItems)),
            _ => None,
        }
    }

    /// Path to the `target` array of the collection.
    pub(crate) fn target(self) -> Path {
        match self {
            Self::Character => characters_target(),
            Self::Items(list) => list.target(),
        }
    }

    /// The key that identifies a row inside its element.
    pub(crate) fn row_key(self) -> &'static str {
        match self {
            Self::Character => "id",
            Self::Items(_) => "contentId",
        }
    }
}

/// Path to `ownedCharacterList.target`.
pub(crate) fn characters_target() -> Path {
    vec![
        Segment::Key("userData"),
        Segment::Doc,
        Segment::Key("ownedCharacterList"),
        Segment::Doc,
        Segment::Key("target"),
    ]
}

/// Path to the document of the element `index` of `target`.
pub(crate) fn element(mut target: Path, index: usize) -> Path {
    target.push(Segment::Index(index));
    target.push(Segment::Doc);
    target
}

/// `path` followed by `tail`.
pub(crate) fn with(mut path: Path, tail: &[Segment<'static>]) -> Path {
    path.extend_from_slice(tail);
    path
}
