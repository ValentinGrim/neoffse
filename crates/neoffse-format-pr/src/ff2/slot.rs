//! `Ff2Model` as the model of the shared slot layer.

use super::Ff2Model;
use crate::shared::impl_slot_model;

impl_slot_model!(Ff2Model, game: super::GAME, capabilities: super::CAPABILITIES, document: "Ff2Document");
