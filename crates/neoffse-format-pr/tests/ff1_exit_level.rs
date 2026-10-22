//! The exit proof of the game interface for a level edit: it changes the
//! level token and the experience token, and no other byte
//! (`docs/formats/pixel-remaster.md#level-up-in-ff-i`).
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use common::edit_proof::{How, Proof, Subject, ViewChange, prove};
use common::ff1_save::{SlotData, character_field};
use common::json_diff::value;
use neoffse_core::{Edit, Value};
use neoffse_format_pr::container::Container;
use neoffse_format_pr::ff1::names;

#[test]
fn the_game_interface_moves_the_experience_with_the_level() {
    let level_path = "userData.ownedCharacterList.target[0].parameter.addtionalLevel";
    let exp_path = "userData.ownedCharacterList.target[0].currentExp";
    let level = character_field(1, names::LEVEL);
    let expected = Proof {
        changes: vec![value(level_path), value(exp_path)],
        view: vec![
            ViewChange::Set(level.clone(), Value::Int(40)),
            ViewChange::Set(character_field(1, names::EXP), Value::Int(207_299)),
        ],
        texts: vec![(level_path.to_owned(), "40".to_owned()), (exp_path.to_owned(), "207299".to_owned())],
    };
    let subject = Subject::fixture(&SlotData::standard(), Container::game());
    prove(&subject, "level", &[Edit::set(level, Value::Int(40))], How::Game, &expected);
}
