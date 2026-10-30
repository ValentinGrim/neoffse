//! An independent restatement of the level-up rule of
//! `docs/formats/pixel-remaster.md#level-up-in-ff-i`, written from the
//! document and sharing no code with the production growth step.
//!
//! The oracle runs the two extreme sources: every roll fails (the bonus is
//! 20 when `hp_value1` forces it), or every roll succeeds with a bonus of 24.

use neoffse_core::{FieldId, Value};
use neoffse_data::GameTables;
use neoffse_format_pr::ff1::{Ff1Model, names};

use super::ff1_save::character_field;

/// What a growth step moves, as the character stores it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Stored {
    pub max_hp_gain: i64,
    pub power: i64,
    pub vitality: i64,
    pub agility: i64,
    pub intelligence: i64,
    pub spirit: i64,
    pub luck: i64,
    pub magic: i64,
    pub accuracy: i64,
    pub evasion: i64,
    pub charges: [i64; 8],
}

fn int(model: &mut Ff1Model, field: &FieldId) -> i64 {
    match model.get(field).unwrap() {
        Value::Int(number) => number,
        other => panic!("{field}: {other:?}"),
    }
}

impl Stored {
    /// Reads the character through `get`.
    pub(crate) fn read(model: &mut Ff1Model, id: u32) -> Stored {
        let mut field = |name| int(model, &character_field(id, name));
        let mut stored = Stored {
            max_hp_gain: field(names::MAX_HP_GAIN),
            power: field(names::POWER),
            vitality: field(names::VITALITY),
            agility: field(names::AGILITY),
            intelligence: field(names::INTELLIGENCE),
            spirit: field(names::SPIRIT),
            luck: field(names::LUCK),
            magic: field(names::MAGIC),
            accuracy: field(names::ACCURACY_GAIN),
            evasion: field(names::MAGIC_EVASION_GAIN),
            charges: [0; 8],
        };
        for level in 1..=8 {
            let at = usize::try_from(level - 1).unwrap();
            stored.charges[at] = int(model, &character_field(id, names::CHARGES_MAX_GAIN).with_id(level));
        }
        stored
    }

    /// The fields in the order of `names::character_fields`, with their
    /// values.
    pub(crate) fn fields(&self, id: u32) -> Vec<(FieldId, i64)> {
        let scalars = [
            (names::MAX_HP_GAIN, self.max_hp_gain),
            (names::POWER, self.power),
            (names::VITALITY, self.vitality),
            (names::AGILITY, self.agility),
            (names::INTELLIGENCE, self.intelligence),
            (names::SPIRIT, self.spirit),
            (names::LUCK, self.luck),
            (names::MAGIC, self.magic),
            (names::ACCURACY_GAIN, self.accuracy),
            (names::MAGIC_EVASION_GAIN, self.evasion),
        ];
        let mut fields: Vec<_> = scalars.into_iter().map(|(name, value)| (character_field(id, name), value)).collect();
        for (level, value) in (1..=8).zip(self.charges) {
            fields.push((character_field(id, names::CHARGES_MAX_GAIN).with_id(level), value));
        }
        fields
    }
}

/// A stat grows by its column, or by 1 when the column is 0 and the roll succeeds.
fn grow_stat(stat: &mut i64, column: u32, succeed: bool) {
    *stat += if column > 0 { i64::from(column) } else { i64::from(succeed) };
}

/// The values after the steps from level `from + 1` to `to`, level 1 having
/// no row. `base_vitality` is the `character_status.vitality` of the
/// character, `job` its job at the end.
pub(crate) fn grow(tables: &GameTables, start: &Stored, base_vitality: u32, job: u32, from: u32, to: u32, succeed: bool) -> Stored {
    let mut now = start.clone();
    for level in (from.max(1) + 1)..=to {
        let row = tables.growth_step(job, level).expect("a growth row for the level");
        let sta = i64::from(base_vitality) + now.vitality;
        now.max_hp_gain += sta / 4 + 1;
        if row.strong_hp || succeed {
            now.max_hp_gain += if succeed { 24 } else { 20 };
        }
        let [strength, vitality, agility, intelligence, spirit, magic, luck] = row.stats;
        grow_stat(&mut now.power, strength, succeed);
        grow_stat(&mut now.vitality, vitality, succeed);
        grow_stat(&mut now.agility, agility, succeed);
        grow_stat(&mut now.intelligence, intelligence, succeed);
        grow_stat(&mut now.spirit, spirit, succeed);
        grow_stat(&mut now.magic, magic, succeed);
        grow_stat(&mut now.luck, luck, succeed);
        now.accuracy += i64::from(row.accuracy_rate);
        now.evasion += i64::from(row.magic_evasion_rate);
        for (held, column) in now.charges.iter_mut().zip(row.charges) {
            *held = if job == 7 || job == 8 {
                (*held + i64::from(column)).min(4)
            } else {
                i64::from(column)
            };
        }
    }
    now
}
