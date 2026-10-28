//! The growth step of a level increase under the game interface, computed by
//! hand from the standard slot and the rows of group 7, 8 and 10 of
//! `data/pr/ff1/growth.toml` (`docs/formats/pixel-remaster.md#level-up-in-ff-i`).
//! `edits` is the forward set of the change, in order.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use common::ff1_save::{SlotData, character_field, model, standard_model};
use common::rolls::Script;
use neoffse_core::{Edit, FieldId, Value};
use neoffse_format_pr::ff1::{Ff1Model, GAME, names};

fn set(field: FieldId, value: i64) -> Edit {
    Edit::set(field, Value::Int(value))
}

fn field(id: u32, name: &'static str) -> FieldId {
    character_field(id, name)
}

/// `charges_max_gain.<level>` of character `id`.
fn charge(id: u32, level: u32) -> FieldId {
    field(id, names::CHARGES_MAX_GAIN).with_id(level)
}

fn int(model: &mut Ff1Model, field: &FieldId) -> i64 {
    match model.get(field).unwrap() {
        Value::Int(value) => value,
        other => panic!("{field}: {other:?}"),
    }
}

/// `Set level` of character `id` under `script`; returns the forward edits.
fn raise(model: &mut Ff1Model, id: u32, level: i64, script: &mut Script) -> Vec<Edit> {
    let change = model.apply_game_with(&set(field(id, names::LEVEL), level), script).unwrap();
    change.forward.edits().to_vec()
}

#[test]
fn knight_three_levels_all_rolls_fail() {
    let mut model = standard_model();
    let mut script = Script::fixed(false, 20);
    let edits = raise(&mut model, 1, 34, &mut script);
    assert_eq!(
        edits,
        [
            set(field(1, names::LEVEL), 34),
            set(field(1, names::EXP), 125_357),
            set(field(1, names::MAX_HP_GAIN), 461),
            set(field(1, names::POWER), 13),
            set(field(1, names::VITALITY), 22),
            set(field(1, names::AGILITY), 32),
            set(field(1, names::LUCK), 62),
            set(field(1, names::ACCURACY_GAIN), 155),
            set(field(1, names::MAGIC_EVASION_GAIN), 142),
            set(charge(1, 1), 4),
            set(charge(1, 2), 4),
            set(charge(1, 3), 4),
            set(charge(1, 6), 4),
            set(charge(1, 7), 4),
        ]
    );
    assert_eq!(script.drawn(), (19, 1));
    assert_eq!(int(&mut model, &field(1, names::MAX_HP_GAIN)), 461);
}

#[test]
fn knight_three_levels_all_rolls_succeed() {
    let mut model = standard_model();
    let mut script = Script::fixed(true, 24);
    let edits = raise(&mut model, 1, 34, &mut script);
    let wanted = [
        (names::MAX_HP_GAIN, 513),
        (names::POWER, 14),
        (names::VITALITY, 24),
        (names::AGILITY, 34),
        (names::INTELLIGENCE, 44),
        (names::SPIRIT, 54),
        (names::LUCK, 64),
        (names::MAGIC, 74),
        (names::ACCURACY_GAIN, 155),
        (names::MAGIC_EVASION_GAIN, 142),
    ];
    let mut expected = vec![set(field(1, names::LEVEL), 34), set(field(1, names::EXP), 125_357)];
    expected.extend(wanted.map(|(name, value)| set(field(1, name), value)));
    expected.extend([1, 2, 3, 6, 7].map(|level| set(charge(1, level), 4)));
    assert_eq!(edits, expected);
    assert_eq!(script.drawn(), (19, 3));
}

#[test]
fn ninja_two_levels_all_rolls_fail() {
    let mut model = standard_model();
    let mut script = Script::fixed(false, 20);
    let edits = raise(&mut model, 2, 34, &mut script);
    assert_eq!(
        edits,
        [
            set(field(2, names::LEVEL), 34),
            set(field(2, names::EXP), 125_357),
            set(field(2, names::MAX_HP_GAIN), 456),
            set(field(2, names::POWER), 13),
            set(field(2, names::AGILITY), 33),
            set(field(2, names::LUCK), 63),
            set(field(2, names::ACCURACY_GAIN), 156),
            set(field(2, names::MAGIC_EVASION_GAIN), 142),
            set(charge(2, 2), 4),
            set(charge(2, 3), 1),
            set(charge(2, 4), 3),
            set(charge(2, 6), 4),
        ]
    );
    assert_eq!(script.drawn(), (13, 1));
}

#[test]
fn a_mage_job_replaces_the_charges() {
    let mut data = SlotData::standard();
    data.characters[2].job = 10;
    data.characters[2].level = "1".to_owned();
    let mut model = model(&data.render());
    let mut script = Script::fixed(false, 20);
    let edits = raise(&mut model, 3, 3, &mut script);
    let mut expected = vec![
        set(field(3, names::LEVEL), 3),
        set(field(3, names::EXP), 42),
        set(field(3, names::MAX_HP_GAIN), 449),
        set(field(3, names::POWER), 14),
        set(field(3, names::AGILITY), 34),
        set(field(3, names::ACCURACY_GAIN), 155),
        set(field(3, names::MAGIC_EVASION_GAIN), 145),
    ];
    expected.extend([1, 2, 0, 0, 0, 0, 0, 0].into_iter().zip(1..).map(|(value, level)| set(charge(3, level), value)));
    assert_eq!(edits, expected);
    assert_eq!(script.drawn(), (14, 0));
}

#[test]
fn the_draw_order_follows_the_rule_not_the_view() {
    let mut model = standard_model();
    // The HP roll first (consumed although the row forces the bonus), then
    // vitality, agility, intelligence, spirit, magic, luck.
    let mut script = Script::queued(&[true, false, true, false, false, true, false], &[23]);
    let edits = raise(&mut model, 1, 32, &mut script);
    let mut expected = vec![
        set(field(1, names::LEVEL), 32),
        set(field(1, names::EXP), 103_811),
        set(field(1, names::MAX_HP_GAIN), 444),
        set(field(1, names::POWER), 12),
        set(field(1, names::AGILITY), 32),
        set(field(1, names::MAGIC), 72),
        set(field(1, names::ACCURACY_GAIN), 147),
        set(field(1, names::MAGIC_EVASION_GAIN), 136),
    ];
    expected.extend([1, 2, 3, 6, 7].map(|level| set(charge(1, level), 4)));
    assert_eq!(edits, expected);
    script.finished();
}

#[test]
fn sta_is_read_before_the_steps_own_gain() {
    let mut data = SlotData::standard();
    data.characters[0].stats[1] = 24;
    let mut model = model(&data.render());
    let mut script = Script::fixed(true, 24);
    raise(&mut model, 1, 33, &mut script);
    // Step 32: STA 15 + 24, 39 / 4 + 1 = 10, bonus 24. Step 33: STA 15 + 25.
    assert_eq!(int(&mut model, &field(1, names::MAX_HP_GAIN)), 411 + 34 + 35);
    assert_eq!(int(&mut model, &field(1, names::VITALITY)), 26);
    assert_eq!(int(&mut model, &field(1, names::EXP)), 114_248);
}

#[test]
fn level_one_has_no_growth_row() {
    let mut data = SlotData::standard();
    data.characters[0].level = "0".to_owned();
    let mut model = model(&data.render());
    let mut script = Script::fixed(false, 20);
    let edits = raise(&mut model, 1, 3, &mut script);
    let mut expected = vec![
        set(field(1, names::LEVEL), 3),
        set(field(1, names::EXP), 42),
        set(field(1, names::MAX_HP_GAIN), 451),
        set(field(1, names::POWER), 12),
        set(field(1, names::VITALITY), 22),
        set(field(1, names::AGILITY), 32),
        set(field(1, names::ACCURACY_GAIN), 151),
        set(field(1, names::MAGIC_EVASION_GAIN), 139),
    ];
    expected.extend([1, 2, 3, 6, 7].map(|level| set(charge(1, level), 4)));
    assert_eq!(edits, expected);
    assert_eq!(script.drawn(), (13, 1));
}

#[test]
fn a_source_outside_20_to_24_is_held_to_the_range() {
    for (answer, gain) in [(99, 445), (0, 441)] {
        let mut model = standard_model();
        raise(&mut model, 1, 32, &mut Script::fixed(false, answer));
        assert_eq!(int(&mut model, &field(1, names::MAX_HP_GAIN)), gain, "bonus {answer}");
    }
}

/// STA is `character_status.vitality` plus `addtionalVitality`, and the HP step
/// applies no cap to it (`docs/formats/pixel-remaster.md#level-up-in-ff-i`).
#[test]
fn the_hp_step_reads_stamina_past_the_stat_cap() {
    let mut data = SlotData::standard();
    data.characters[0].stats[1] = 200;
    let mut model = model(&data.render());
    let tables = neoffse_data::tables_for(GAME).unwrap();
    let base = i64::from(tables.base_vitality(1).unwrap());
    let sta = base + 200;
    assert!(sta > 99, "the stamina passes the stat cap");
    let before = int(&mut model, &field(1, names::MAX_HP_GAIN));
    raise(&mut model, 1, 32, &mut Script::fixed(false, 20));
    let strong = tables.growth_step(7, 32).unwrap().strong_hp;
    let bonus = if strong { 20 } else { 0 };
    assert_eq!(int(&mut model, &field(1, names::MAX_HP_GAIN)) - before, sta / 4 + 1 + bonus);
}
