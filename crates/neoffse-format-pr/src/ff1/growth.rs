//! The growth step of a level increase under the game interface.
//!
//! A Level or Experience edit that raises the level plans one step per level
//! gained, in the order and with the draws that
//! `docs/formats/pixel-remaster.md#level-up-in-ff-i` states (the order of the
//! draws is also in the module docs of [`Rolls`]). The steps run in memory on
//! the values read once from the document, and the plan holds one `Set` per
//! field whose final value differs from the stored one. The game stores the
//! raw sums and clamps nothing on write, so these edits apply under
//! `CapPolicy::StorageOnly` (see `apply_game_with`).

use neoffse_core::{Edit, EditError, FieldId, Value};
use neoffse_data::GrowthStep;

use super::level::character_field;
use super::{Ff1Model, Rolls, names};
use crate::document::Segment;
use crate::shared::paths::{Collection, characters_target, element, with};
use crate::shared::read::{find_row, format_error, missing_row, read_int};

/// The Knight job id. Knight and Ninja add the charges of a row and hold every
/// value at [`KNIGHT_NINJA_CHARGE_CAP`]; every other job replaces the values
/// with the row (`docs/formats/pixel-remaster.md#level-up-in-ff-i`, step 4).
const KNIGHT_JOB: i64 = 7;
/// The Ninja job id.
const NINJA_JOB: i64 = 8;
/// The charge cap of Knight and Ninja. It is a constant of the game code
/// (`KnightMaxMagticCount`, `NinjaMaxMagicCount`); no master file holds it.
const KNIGHT_NINJA_CHARGE_CAP: i64 = 4;

/// The strong HP bonus runs from 20 to 24 (`#level-up-in-ff-i`, "Random
/// parts"). A source that answers outside the range is held to it.
const BONUS_RANGE: (u32, u32) = (20, 24);

/// The scalar fields a step moves, in `names::character_fields` order. The
/// eight `charges_max_gain` entries follow.
const SCALARS: [&str; 10] = [
    names::MAX_HP_GAIN,
    names::POWER,
    names::VITALITY,
    names::AGILITY,
    names::INTELLIGENCE,
    names::SPIRIT,
    names::LUCK,
    names::MAGIC,
    names::ACCURACY_GAIN,
    names::MAGIC_EVASION_GAIN,
];

/// The running values of a character, in the order of [`SCALARS`] and then
/// `charges_max_gain.1` to `.8`.
type Values = [i64; 18];

const HP: usize = 0;
const VITALITY: usize = 2;
const ACCURACY: usize = 8;
const EVASION: usize = 9;
const CHARGES: usize = 10;
/// The slot of each stat in [`Values`], in the rule's order: strength,
/// vitality, agility, intelligence, spirit, magic, luck. The field order has
/// luck before magic.
const RULE_SLOTS: [usize; 7] = [1, 2, 3, 4, 5, 7, 6];

/// One step of `docs/formats/pixel-remaster.md#level-up-in-ff-i`, in the
/// order of the draws.
fn step(values: &mut Values, row: &GrowthStep, base_vitality: u32, job: i64, rolls: &mut dyn Rolls) {
    // STA is read before this step's own gains.
    let sta = i64::from(base_vitality).saturating_add(values[VITALITY]);
    let mut hp = sta.max(0) / 4 + 1;
    let hit = rolls.one_in_eight();
    if row.strong_hp || hit {
        let (low, high) = BONUS_RANGE;
        hp = hp.saturating_add(i64::from(rolls.hp_bonus().clamp(low, high)));
    }
    values[HP] = values[HP].saturating_add(hp);
    for (slot, column) in RULE_SLOTS.into_iter().zip(row.stats) {
        let gain = if column == 0 { u32::from(rolls.one_in_eight()) } else { column };
        values[slot] = values[slot].saturating_add(i64::from(gain));
    }
    values[ACCURACY] = values[ACCURACY].saturating_add(i64::from(row.accuracy_rate));
    values[EVASION] = values[EVASION].saturating_add(i64::from(row.magic_evasion_rate));
    let capped = job == KNIGHT_JOB || job == NINJA_JOB;
    for (value, column) in values[CHARGES..].iter_mut().zip(row.charges) {
        *value = if capped {
            let sum = value.saturating_add(i64::from(column));
            if sum >= KNIGHT_NINJA_CHARGE_CAP { KNIGHT_NINJA_CHARGE_CAP } else { sum }
        } else {
            i64::from(column)
        };
    }
}

impl Ff1Model {
    /// The `Set` edits of the growth steps of character `id` up to level `to`.
    ///
    /// Empty when `to` does not pass the stored level: a decrease or an equal
    /// level changes no stat and draws nothing. The steps start at level 2,
    /// because `growth_curve.csv` has no row for level 1.
    ///
    /// `UnknownId` for a job without a growth row (field `character.<id>.job`)
    /// and for a status without a base vitality (field `character.<id>.level`).
    /// Nothing is written.
    pub(super) fn growth_edits(&mut self, id: u32, to: i64, rolls: &mut dyn Rolls) -> Result<Vec<Edit>, EditError> {
        let from = self.stored_int(&character_field(id, names::LEVEL))?;
        if to <= from {
            return Ok(Vec::new());
        }
        let job_field = character_field(id, names::JOB);
        let job = self.stored_int(&job_field)?;
        let status = self.status_of(id)?;
        let level_field = character_field(id, names::LEVEL);
        let base_vitality = u32::try_from(status)
            .ok()
            .and_then(|status| self.tables.base_vitality(status))
            .ok_or(EditError::UnknownId {
                field: level_field,
                id: status,
            })?;
        let fields = gain_fields(id);
        let mut old = [0; 18];
        for (slot, field) in old.iter_mut().zip(&fields) {
            *slot = self.stored_int(field)?;
        }
        let mut values = old;
        // from < to <= the last table level here, so both bounds fit a u32.
        let last = u32::try_from(to).unwrap_or(u32::MAX);
        for level in u32::try_from(from.max(1).saturating_add(1)).unwrap_or(u32::MAX)..=last {
            let row = u32::try_from(job).ok().and_then(|job| self.tables.growth_step(job, level));
            let row = row.ok_or_else(|| EditError::UnknownId {
                field: job_field.clone(),
                id: job,
            })?;
            step(&mut values, &row, base_vitality, job, rolls);
        }
        Ok(fields
            .into_iter()
            .zip(old.into_iter().zip(values))
            .filter(|(_, (before, after))| before != after)
            .map(|(field, (_, after))| Edit::set(field, Value::Int(after)))
            .collect())
    }

    /// The integer at `field`; another kind of value is `ReadOnly`, as in the
    /// linked step of a Level or Experience edit.
    fn stored_int(&mut self, field: &FieldId) -> Result<i64, EditError> {
        match self.get(field)? {
            Value::Int(number) => Ok(number),
            _ => Err(EditError::ReadOnly { field: field.clone() }),
        }
    }

    /// The `characterStatusId` of character `id`. A missing key or another
    /// kind of value is the `Format` error of the level field.
    fn status_of(&mut self, id: u32) -> Result<i64, EditError> {
        let field = character_field(id, names::LEVEL);
        let index = find_row(&mut self.document, Collection::Character, id)
            .map_err(|source| format_error(&field, source))?
            .ok_or_else(|| missing_row(names::CHARACTER, id))?;
        let path = with(element(characters_target(), index), &[Segment::Key("characterStatusId")]);
        read_int(&mut self.document, &path).map_err(|source| format_error(&field, source))
    }
}

/// The fields a step can move, in [`Values`] order.
fn gain_fields(id: u32) -> Vec<FieldId> {
    let scalars = SCALARS.into_iter().map(|name| character_field(id, name));
    let charges = (1..=names::SPELL_LEVELS).map(|level| character_field(id, names::CHARGES_MAX_GAIN).with_id(level));
    scalars.chain(charges).collect()
}
