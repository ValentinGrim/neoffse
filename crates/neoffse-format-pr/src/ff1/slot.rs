//! `Ff1Model` as the model of the shared slot layer.

use super::Ff1Model;
use crate::shared::impl_slot_model;

impl_slot_model!(Ff1Model, game: super::GAME, capabilities: super::CAPABILITIES, document: "Ff1Document");
