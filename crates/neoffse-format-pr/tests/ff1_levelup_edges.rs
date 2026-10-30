//! The edges of the growth step under the game interface: what draws nothing,
//! what the caps leave alone, what refuses, and the undo
//! (`docs/formats/pixel-remaster.md#level-up-in-ff-i`).
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use common::ff1_save::{SlotData, character_field, model, standard_model};
use common::growth_oracle::Stored;
use common::rolls::Script;
use neoffse_core::{CapPolicy, Edit, EditError, FieldId, Value};
use neoffse_data::Language;
use neoffse_format_pr::ff1::{Ff1Model, Rolls, names};

/// A change to the standard slot before the edit.
type Twist = fn(&mut SlotData);

fn set(field: FieldId, value: i64) -> Edit {
    Edit::set(field, Value::Int(value))
}

fn field(id: u32, name: &'static str) -> FieldId {
    character_field(id, name)
}

fn level(id: u32) -> FieldId {
    field(id, names::LEVEL)
}

fn int(model: &mut Ff1Model, field: &FieldId) -> i64 {
    match model.get(field).unwrap() {
        Value::Int(value) => value,
        other => panic!("{field}: {other:?}"),
    }
}

fn edits(model: &mut Ff1Model, edit: &Edit, rolls: &mut dyn Rolls) -> Vec<Edit> {
    model.apply_game_with(edit, rolls).unwrap().forward.edits().to_vec()
}

/// Every field of the view with its value, minus the fields in `skip`.
fn flat(model: &mut Ff1Model, skip: &[&str]) -> Vec<(FieldId, Value)> {
    let view = model.view(Language::En).unwrap();
    let name_of = |id: &FieldId| id.to_string().rsplit('.').next().unwrap_or_default().to_owned();
    view.sections
        .iter()
        .flat_map(|section| &section.entities)
        .flat_map(|entity| &entity.fields)
        .filter(|view| !skip.contains(&name_of(&view.id).as_str()))
        .map(|view| (view.id.clone(), view.value.clone()))
        .collect()
}

#[test]
fn a_level_decrease_draws_nothing_and_moves_no_stat() {
    let mut model = standard_model();
    let before = flat(&mut model, &["level", "exp", "next_level"]);
    let mut none = Script::queued(&[], &[]);
    let plan = edits(&mut model, &set(level(1), 1), &mut none);
    assert_eq!(plan, [set(level(1), 1), set(field(1, names::EXP), 0)]);
    assert_eq!(flat(&mut model, &["level", "exp", "next_level"]), before);
    let mut model = standard_model();
    let plan = edits(&mut model, &set(level(1), 31), &mut none);
    assert_eq!(plan, [set(level(1), 31), set(field(1, names::EXP), 94_025)]);
    assert_eq!(none.drawn(), (0, 0));
}

fn growth_part(plan: &[Edit]) -> &[Edit] {
    &plan[2..]
}

#[test]
fn an_experience_edit_grows_like_the_level_edit() {
    for (eighth, bonus) in [(false, 20), (true, 24)] {
        let mut by_level = standard_model();
        let leveled = edits(&mut by_level, &set(level(1), 34), &mut Script::fixed(eighth, bonus));
        let mut by_exp = standard_model();
        let expd = edits(&mut by_exp, &set(field(1, names::EXP), 130_000), &mut Script::fixed(eighth, bonus));
        assert_eq!(expd[..2], [set(field(1, names::EXP), 130_000), set(level(1), 34)]);
        assert_eq!(growth_part(&expd), growth_part(&leveled), "rolls {eighth}");
        let skip = ["exp", "next_level"];
        assert_eq!(flat(&mut by_level, &skip), flat(&mut by_exp, &skip), "rolls {eighth}");
        assert_eq!(int(&mut by_level, &field(1, names::EXP)), 125_357);
        assert_eq!(int(&mut by_exp, &field(1, names::EXP)), 130_000);
    }
}

#[test]
fn an_experience_edit_inside_the_level_or_below_grows_nothing() {
    let mut none = Script::queued(&[], &[]);
    let mut model = standard_model();
    assert_eq!(
        edits(&mut model, &set(field(1, names::EXP), 100_200), &mut none),
        [set(field(1, names::EXP), 100_200)]
    );
    let mut model = standard_model();
    let plan = edits(&mut model, &set(field(1, names::EXP), 0), &mut none);
    assert_eq!(plan, [set(field(1, names::EXP), 0), set(level(1), 1)]);
    assert_eq!(none.drawn(), (0, 0));
}

#[test]
fn a_cap_does_not_stop_the_game_from_growing() {
    let mut data = SlotData::standard();
    data.characters[0].stats[0] = 99;
    let mut model = model(&data.render());
    edits(&mut model, &set(level(1), 32), &mut Script::fixed(false, 20));
    assert_eq!(int(&mut model, &field(1, names::POWER)), 100);
    let mut plain = standard_model();
    let before = plain.document().to_json();
    let refused = plain.apply_game(&set(field(1, names::POWER), 100));
    assert!(matches!(refused, Err(EditError::AboveCap { cap: 99, value: 100, .. })), "{refused:?}");
    assert_eq!(plain.document().to_json(), before);
}

#[test]
fn a_value_past_the_storage_bound_refuses_the_whole_edit() {
    let mut data = SlotData::standard();
    data.characters[0].max_hp_gain = i32::MAX.try_into().unwrap();
    let mut model = model(&data.render());
    let before = model.document().to_json();
    let refused = model.apply_game_with(&set(level(1), 32), &mut Script::fixed(false, 20));
    let max_hp_gain = field(1, names::MAX_HP_GAIN);
    assert!(
        matches!(&refused, Err(EditError::OutOfRange { field, .. }) if *field == max_hp_gain),
        "{refused:?}"
    );
    assert_eq!(model.document().to_json(), before);
}

#[test]
fn a_job_or_status_without_a_growth_row_is_refused() {
    let job_field = field(1, names::JOB);
    let broken: [(Twist, EditError); 3] = [
        (
            |data| data.characters[0].job = 13,
            EditError::UnknownId {
                field: job_field.clone(),
                id: 13,
            },
        ),
        (|data| data.characters[0].job = 0, EditError::UnknownId { field: job_field, id: 0 }),
        (
            |data| data.characters[0].status_id = Some("9".to_owned()),
            EditError::UnknownId { field: level(1), id: 9 },
        ),
    ];
    for (twist, wanted) in broken {
        let mut data = SlotData::standard();
        twist(&mut data);
        let mut model = model(&data.render());
        let before = model.document().to_json();
        let refused = model.apply_game_with(&set(level(1), 40), &mut Script::queued(&[], &[]));
        assert_eq!(refused.unwrap_err().to_string(), wanted.to_string());
        assert_eq!(model.document().to_json(), before);
        let plan = edits(&mut model, &set(level(1), 1), &mut Script::queued(&[], &[]));
        assert_eq!(plan.len(), 2);
    }
}

#[test]
fn a_missing_status_key_is_a_format_error() {
    let mut data = SlotData::standard();
    data.characters[0].status_id = None;
    let mut model = model(&data.render());
    let before = model.document().to_json();
    let refused = model.apply_game_with(&set(level(1), 40), &mut Script::queued(&[], &[]));
    assert!(matches!(refused, Err(EditError::Format { .. })), "{refused:?}");
    assert_eq!(model.document().to_json(), before);
    let plan = edits(&mut model, &set(level(1), 1), &mut Script::queued(&[], &[]));
    assert_eq!(plan.len(), 2);
}

#[test]
fn undo_gives_the_document_back_byte_for_byte() {
    let mut model = standard_model();
    let before = model.document().to_json();
    let change = model.apply_game_with(&set(level(1), 34), &mut Script::fixed(false, 20)).unwrap();
    let after = model.document().to_json();
    let apply = |model: &mut Ff1Model, set: &neoffse_core::ChangeSet| {
        set.apply_with(CapPolicy::StorageOnly, |step, policy| model.apply(step, policy)).unwrap();
    };
    apply(&mut model, &change.inverse);
    assert_eq!(model.document().to_json(), before);
    apply(&mut model, &change.forward);
    assert_eq!(model.document().to_json(), after);

    // Levels rewrite compact on a spaced slot: the view is what must return.
    let mut data = SlotData::standard();
    data.spaced = true;
    let mut spaced = model_of(&data);
    let view = spaced.view(Language::En).unwrap();
    let change = spaced.apply_game_with(&set(level(1), 34), &mut Script::fixed(false, 20)).unwrap();
    let grown = spaced.view(Language::En).unwrap();
    assert_ne!(grown, view);
    apply(&mut spaced, &change.inverse);
    assert_eq!(spaced.view(Language::En).unwrap(), view);
    apply(&mut spaced, &change.forward);
    assert_eq!(spaced.view(Language::En).unwrap(), grown);
}

fn model_of(data: &SlotData) -> Ff1Model {
    model(&data.render())
}

/// A xorshift source: a different mix of answers for each seed.
struct Xorshift(u64);

impl Xorshift {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
}

impl Rolls for Xorshift {
    fn one_in_eight(&mut self) -> bool {
        self.next().is_multiple_of(8)
    }

    fn hp_bonus(&mut self) -> u32 {
        20 + u32::try_from(self.next() % 5).unwrap()
    }
}

#[test]
fn any_mix_of_rolls_stays_between_the_two_extremes() {
    let grown = |rolls: &mut dyn Rolls| {
        let mut model = standard_model();
        model.apply_game_with(&set(level(1), 34), rolls).unwrap();
        Stored::read(&mut model, 1)
    };
    let low = grown(&mut Script::fixed(false, 20));
    let high = grown(&mut Script::fixed(true, 24));
    for seed in 1..=64_u64 {
        let mix = grown(&mut Xorshift(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1));
        let between = |name: &str, value: i64, low: i64, high: i64| {
            assert!((low..=high).contains(&value), "seed {seed}: {name} {value} outside {low}..={high}");
        };
        between("max_hp_gain", mix.max_hp_gain, low.max_hp_gain, high.max_hp_gain);
        between("power", mix.power, low.power, high.power);
        between("vitality", mix.vitality, low.vitality, high.vitality);
        between("agility", mix.agility, low.agility, high.agility);
        between("intelligence", mix.intelligence, low.intelligence, high.intelligence);
        between("spirit", mix.spirit, low.spirit, high.spirit);
        between("luck", mix.luck, low.luck, high.luck);
        between("magic", mix.magic, low.magic, high.magic);
        assert_eq!((mix.accuracy, mix.evasion, mix.charges), (low.accuracy, low.evasion, low.charges));
        assert_eq!((high.accuracy, high.evasion, high.charges), (low.accuracy, low.evasion, low.charges));
    }
}

#[test]
fn the_real_source_keeps_the_fixed_parts() {
    let mut model = standard_model();
    model.apply_game(&set(level(1), 34)).unwrap();
    assert_eq!(int(&mut model, &level(1)), 34);
    assert_eq!(int(&mut model, &field(1, names::EXP)), 125_357);
    let stored = Stored::read(&mut model, 1);
    assert_eq!((stored.accuracy, stored.evasion), (155, 142));
    assert_eq!(stored.charges, [4, 4, 4, 1, 3, 4, 4, 1]);
    let inside = |name: &str, value: i64, low: i64, high: i64| assert!((low..=high).contains(&value), "{name} {value}");
    inside("max_hp_gain", stored.max_hp_gain, 461, 513);
    inside("power", stored.power, 13, 14);
    inside("vitality", stored.vitality, 22, 24);
    inside("agility", stored.agility, 32, 34);
    inside("intelligence", stored.intelligence, 41, 44);
    inside("spirit", stored.spirit, 51, 54);
    inside("luck", stored.luck, 62, 64);
    inside("magic", stored.magic, 71, 74);
}

#[test]
fn raw_apply_grows_nothing() {
    for policy in [CapPolicy::Enforce, CapPolicy::StorageOnly] {
        let mut model = standard_model();
        let before = Stored::read(&mut model, 1);
        model.apply(&set(level(1), 40), policy).unwrap();
        assert_eq!(int(&mut model, &level(1)), 40);
        assert_eq!(Stored::read(&mut model, 1), before);
        assert_eq!(int(&mut model, &field(1, names::EXP)), 100_100);
    }
}
