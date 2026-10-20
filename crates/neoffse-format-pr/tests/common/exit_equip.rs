//! The equip oracle: the expected result of `Set character.<id>.equipment.1`
//! computed from the tree before the edit and the equip plan of
//! `Ff1Model::apply_game` (`docs/formats/pixel-remaster.md#equipment-slots`,
//! `#inventory`), without running the model.

use neoffse_core::{Edit, RowId, TableKind, Value};
use neoffse_format_pr::ff1::names;

use super::edit_proof::{Proof, ViewChange};
use super::exit_cases::Case;
use super::exit_facts::{CHARS, Facts, INV};
use super::json_diff::{added, value};

/// Which weapon the case equips.
#[derive(Clone, Copy)]
enum Pick {
    /// The first weapon the inventory does not hold.
    New,
    /// The first weapon the inventory holds.
    Held,
    /// The placeholder of the weapon table.
    Unequip,
}

pub(super) fn equip_cases(facts: &Facts<'_>) -> Vec<(&'static str, Option<Case>)> {
    [("equip_new", Pick::New), ("equip_held", Pick::Held), ("unequip", Pick::Unequip)]
        .map(|(name, pick)| (name, equip(facts, name, pick)))
        .into()
}

/// The weapon id to equip in slot 1, different from the current one.
fn choose(facts: &Facts<'_>, pick: Pick, current: i64) -> Option<u32> {
    let weapons = facts.tables.table(TableKind::Weapon)?;
    let candidate = match pick {
        Pick::Unequip => facts.tables.placeholder(TableKind::Weapon)?.id,
        Pick::New | Pick::Held => {
            let wanted = matches!(pick, Pick::Held);
            weapons
                .iter()
                .find(|entry| !entry.placeholder && i64::from(entry.id) != current && facts.inv_rows.contains(&entry.id) == wanted)?
                .id
        }
    };
    (i64::from(candidate) != current).then_some(candidate)
}

/// The equipment slots, other than slot 1 of the first character, that hold
/// `id`.
fn other_wearers(facts: &Facts<'_>, id: u32) -> usize {
    let tree = &facts.subject.tree;
    let mut found = 0;
    for character in 0..facts.characters {
        let keys = format!("{CHARS}[{character}].equipmentList.keys");
        for slot in 1..=names::EQUIPMENT_SLOTS {
            if character == facts.char_index && slot == 1 {
                continue;
            }
            let Some(index) = tree.find_int(&keys, i64::from(slot)) else { continue };
            let path = format!("{CHARS}[{character}].equipmentList.values[{index}].contentId");
            found += usize::from(tree.int(&path) == Some(i64::from(id)));
        }
    }
    found
}

fn equip(facts: &Facts<'_>, name: &'static str, pick: Pick) -> Option<Case> {
    let tree = &facts.subject.tree;
    let slot_index = tree.find_int(&format!("{}.equipmentList.keys", facts.char_path), 1)?;
    let slot = format!("{}.equipmentList.values[{slot_index}]", facts.char_path);
    let (id_path, count_path) = (format!("{slot}.contentId"), format!("{slot}.count"));
    let old_count = tree.int(&count_path)?;
    let target = choose(facts, pick, tree.int(&id_path)?)?;
    let placeholder = facts.tables.content(target)?.placeholder;
    let slot_field = facts.field(names::EQUIPMENT).with_id(1);
    let held = tree.find_row(INV, "contentId", i64::from(target));
    let row = RowId::new(neoffse_core::FieldId::new(names::INVENTORY), target);
    let stored = |index: usize| tree.int(&format!("{INV}[{index}].count"));
    let needed = i64::try_from(other_wearers(facts, target) + 1).ok()?;

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
        raw: false,
    })
}
