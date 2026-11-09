//! `Ff2Model` as the model of the shared slot layer.

use super::Ff2Model;
use super::names;
use crate::shared::impl_slot_model;

impl_slot_model!(
    Ff2Model,
    game: super::GAME,
    capabilities: super::CAPABILITIES,
    keyed_lists: [names::CHARACTER, names::INVENTORY, names::KEY_ITEMS, names::KEYWORDS],
    document: "Ff2Document"
);
