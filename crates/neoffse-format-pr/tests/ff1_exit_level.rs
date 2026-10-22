//! The exit proof of the game interface for a level edit: a decrease changes
//! the level token and the experience token and no other byte, and a rise
//! also changes the growth tokens of the step
//! (`docs/formats/pixel-remaster.md#level-up-in-ff-i`).
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use common::edit_proof::{How, Proof, Subject, ViewChange, prove};
use common::ff1_save::{SlotData, character_field};
use common::json_diff::value;
use neoffse_core::{Edit, Value};
use neoffse_format_pr::container::Container;
use neoffse_format_pr::ff1::names;

const CHARACTER: &str = "userData.ownedCharacterList.target[0]";

fn subject() -> Subject {
    Subject::fixture(&SlotData::standard(), Container::game())
}

#[test]
fn the_game_interface_moves_the_experience_with_the_level() {
    let level_path = format!("{CHARACTER}.parameter.addtionalLevel");
    let exp_path = format!("{CHARACTER}.currentExp");
    let level = character_field(1, names::LEVEL);
    let expected = Proof {
        changes: vec![value(&level_path), value(&exp_path)],
        view: vec![
            ViewChange::Set(level.clone(), Value::Int(1)),
            ViewChange::Set(character_field(1, names::EXP), Value::Int(0)),
        ],
        texts: vec![(level_path, "1".to_owned()), (exp_path, "0".to_owned())],
    };
    prove(&subject(), "level", &[Edit::set(level, Value::Int(1))], How::Game, &expected);
}

#[test]
fn the_game_interface_grows_the_stats_with_the_level() {
    // Step 32 of group 7: STA 15 + 21 = 36, 36 / 4 + 1 = 10, the forced
    // bonus 20 with every roll failing; the charges 1, 2, 3, 6, 7 reach the
    // Knight cap of 4.
    let parameter = format!("{CHARACTER}.parameter");
    let list = format!("{parameter}.addtionalMaxMpCountList.values");
    let mut paths = vec![
        (format!("{parameter}.addtionalLevel"), character_field(1, names::LEVEL), 32),
        (format!("{CHARACTER}.currentExp"), character_field(1, names::EXP), 103_811),
        (format!("{parameter}.addtionalMaxHp"), character_field(1, names::MAX_HP_GAIN), 441),
        (format!("{parameter}.addtionalPower"), character_field(1, names::POWER), 12),
        (format!("{parameter}.addtionalAccuracyRate"), character_field(1, names::ACCURACY_GAIN), 147),
        (
            format!("{parameter}.addtionalAbilityEvasionRate"),
            character_field(1, names::MAGIC_EVASION_GAIN),
            136,
        ),
    ];
    for (index, level) in [(0, 1), (1, 2), (2, 3), (5, 6), (6, 7)] {
        paths.push((format!("{list}[{index}]"), character_field(1, names::CHARGES_MAX_GAIN).with_id(level), 4));
    }
    let expected = Proof {
        changes: paths.iter().map(|(path, _, _)| value(path)).collect(),
        view: paths
            .iter()
            .map(|(_, field, number)| ViewChange::Set(field.clone(), Value::Int(*number)))
            .collect(),
        texts: paths.iter().map(|(path, _, number)| (path.clone(), number.to_string())).collect(),
    };
    let edit = Edit::set(character_field(1, names::LEVEL), Value::Int(32));
    prove(&subject(), "level_up", &[edit], How::Rolled, &expected);
}
