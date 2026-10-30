//! The equip oracle: the expected result of `Set character.<id>.equipment.<S>`
//! for the five slots, computed from the tree before the edit and the equip
//! plan of `Ff1Model::apply_game`
//! (`docs/formats/pixel-remaster.md#equipment-slots`, `#inventory`), without
//! running the model. A slot whose table or save lacks an ingredient skips.

use neoffse_core::{Edit, RowId, TableKind, Value};
use neoffse_format_pr::ff1::names;

use super::edit_proof::{Proof, ViewChange};
use super::exit_cases::Case;
use super::exit_facts::{CHARS, Facts, INV};
use super::json_diff::{added, value};

/// Which item the case equips.
#[derive(Clone, Copy)]
enum Pick {
    /// The first item of the slot's table that the inventory does not hold.
    New,
    /// The first item of the slot's table that the inventory holds.
    Held,
    /// The placeholder of the slot's table.
    Unequip,
}

/// The table of each equipment slot, slot 1 first.
const SLOT_TABLES: [TableKind; 5] = [TableKind::Weapon, TableKind::Shield, TableKind::Armor, TableKind::Helmet, TableKind::Gloves];

/// Case names by slot: new, held, unequip.
const NAMES: [[&str; 3]; 5] = [
    ["equip_new", "equip_held", "unequip"],
    ["equip_new_s2", "equip_held_s2", "unequip_s2"],
    ["equip_new_s3", "equip_held_s3", "unequip_s3"],
    ["equip_new_s4", "equip_held_s4", "unequip_s4"],
    ["equip_new_s5", "equip_held_s5", "unequip_s5"],
];

pub(super) fn equip_cases(facts: &Facts<'_>) -> Vec<(&'static str, Option<Case>)> {
    let mut cases = Vec::new();
    for (index, (kind, names)) in SLOT_TABLES.into_iter().zip(NAMES).enumerate() {
        let slot = u32::try_from(index + 1).unwrap_or(1);
        for (name, pick) in names.into_iter().zip([Pick::New, Pick::Held, Pick::Unequip]) {
            cases.push((name, equip(facts, name, pick, slot, kind)));
        }
    }
    cases
}

/// The id of `kind` to equip, different from the current one.
fn choose(facts: &Facts<'_>, pick: Pick, kind: TableKind, current: i64) -> Option<u32> {
    let items = facts.tables.table(kind)?;
    let candidate = match pick {
        Pick::Unequip => facts.tables.placeholder(kind)?.id,
        Pick::New | Pick::Held => {
            let wanted = matches!(pick, Pick::Held);
            items
                .iter()
                .find(|entry| !entry.placeholder && i64::from(entry.id) != current && facts.inv_rows.contains(&entry.id) == wanted)?
                .id
        }
    };
    (i64::from(candidate) != current).then_some(candidate)
}

/// The equipment slots, other than `slot` of the first character, that hold
/// `id`.
fn other_wearers(facts: &Facts<'_>, id: u32, own_slot: u32) -> usize {
    let tree = &facts.subject.tree;
    let mut found = 0;
    for character in 0..facts.characters {
        let keys = format!("{CHARS}[{character}].equipmentList.keys");
        for slot in 1..=names::EQUIPMENT_SLOTS {
            if character == facts.char_index && slot == own_slot {
                continue;
            }
            let Some(index) = tree.find_int(&keys, i64::from(slot)) else { continue };
            let path = format!("{CHARS}[{character}].equipmentList.values[{index}].contentId");
            found += usize::from(tree.int(&path) == Some(i64::from(id)));
        }
    }
    found
}

fn equip(facts: &Facts<'_>, name: &'static str, pick: Pick, slot: u32, kind: TableKind) -> Option<Case> {
    let tree = &facts.subject.tree;
    let slot_index = tree.find_int(&format!("{}.equipmentList.keys", facts.char_path), i64::from(slot))?;
    let entry = format!("{}.equipmentList.values[{slot_index}]", facts.char_path);
    let (id_path, count_path) = (format!("{entry}.contentId"), format!("{entry}.count"));
    let old_count = tree.int(&count_path)?;
    let target = choose(facts, pick, kind, tree.int(&id_path)?)?;
    let placeholder = facts.tables.content(target)?.placeholder;
    let slot_field = facts.field(names::EQUIPMENT).with_id(slot);
    let held = tree.find_row(INV, "contentId", i64::from(target));
    let row = RowId::new(neoffse_core::FieldId::new(names::INVENTORY), target);
    let stored = |index: usize| tree.int(&format!("{INV}[{index}].count"));
    let needed = i64::try_from(other_wearers(facts, target, slot) + 1).ok()?;

    let mut proof = Proof {
        changes: vec![value(&id_path)],
        view: vec![ViewChange::Set(slot_field.clone(), Value::Int(i64::from(target)))],
        texts: vec![(id_path, target.to_string())],
    };
    let new_count = match (placeholder, held) {
        (true, None) => old_count,
        (false, None) => {
            let element = format!("{INV}[{}]", facts.inv_len);
            proof.changes.push(added(&element));
            proof.view.push(ViewChange::Insert {
                row: row.clone(),
                position: facts.inv_len,
                count: needed,
            });
            proof.texts.push((element, format!("{{\"contentId\":{target},\"count\":{needed}}}")));
            needed
        }
        (false, Some(index)) if stored(index)? < needed => {
            let path = format!("{INV}[{index}].count");
            proof.changes.push(value(&path));
            proof.view.push(ViewChange::Set(row.field(names::COUNT), Value::Int(needed)));
            proof.texts.push((path, needed.to_string()));
            needed
        }
        (_, Some(index)) => stored(index)?,
    };
    if new_count != old_count {
        proof.changes.push(value(&count_path));
        proof
            .view
            .push(ViewChange::Set(slot_field.clone().with_name(names::COUNT), Value::Int(new_count)));
        proof.texts.push((count_path, new_count.to_string()));
    }
    let edit = Edit::set(slot_field, Value::Int(i64::from(target)));
    Some(Case {
        name,
        edits: vec![edit],
        proof,
        linked: None,
        raw: false,
        rolled: false,
    })
}
