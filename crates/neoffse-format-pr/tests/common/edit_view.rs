//! The view side of a proof: what the editor shows after an edit, relative
//! to what it showed before. Messages carry field ids and never a value.

use std::collections::BTreeMap;

use neoffse_core::{FieldId, Key, RowId, SaveView, Value};
use neoffse_format_pr::ff1::names::{self, CHARACTER, COUNT, EQUIPMENT, INVENTORY};

/// What the view must show after an edit, relative to the view before.
#[derive(Debug, Clone)]
pub(crate) enum ViewChange {
    Set(FieldId, Value),
    /// `position`: the index among the rows of that collection.
    Insert {
        row: RowId,
        position: usize,
        count: i64,
    },
    Remove(RowId),
}

type Fields = BTreeMap<FieldId, Value>;
type Order = BTreeMap<&'static str, Vec<FieldId>>;

/// The first way the view `after` departs from `before` with `changes`
/// applied: a lost, gained or different field, a different row order or a
/// stale label of a renamed character.
pub(super) fn mismatch(before: &SaveView, after: &SaveView, changes: &[ViewChange]) -> Option<String> {
    let (mut fields, mut order) = flatten(before);
    for change in changes {
        match change {
            ViewChange::Set(id, value) => {
                fields.insert(id.clone(), value.clone());
            }
            ViewChange::Insert { row, position, count } => {
                fields.insert(row.field(COUNT), Value::Int(*count));
                let rows = order.entry(row.id().section()).or_default();
                rows.insert((*position).min(rows.len()), row.id().clone());
            }
            ViewChange::Remove(row) => {
                fields.retain(|id, _| !id.starts_with(row.id()));
                if let Some(rows) = order.get_mut(row.id().section()) {
                    rows.retain(|id| id != row.id());
                }
            }
        }
    }
    let (actual, actual_order) = flatten(after);
    for (id, expected) in &fields {
        match actual.get(id) {
            None => return Some(format!("view lost {id}")),
            Some(found) if found != expected => return Some(format!("view differs at {id}")),
            Some(_) => {}
        }
    }
    if let Some(id) = actual.keys().find(|id| !fields.contains_key(*id)) {
        return Some(format!("view gained {id}"));
    }
    if let Some((collection, _)) = order.iter().find(|(collection, rows)| actual_order.get(*collection) != Some(rows)) {
        return Some(format!("row order differs in {collection}"));
    }
    stale_label(after, changes)
}

/// A name edit shows in the label of the Party entity.
fn stale_label(view: &SaveView, changes: &[ViewChange]) -> Option<String> {
    changes.iter().find_map(|change| {
        let ViewChange::Set(field, Value::Text(text)) = change else { return None };
        let [Key::Name(CHARACTER), Key::Id(_), Key::Name(names::NAME)] = field.keys() else {
            return None;
        };
        let entity = view
            .sections
            .iter()
            .flat_map(|section| &section.entities)
            .find(|entity| field.parent().is_some_and(|id| entity.id == id));
        entity.is_none_or(|entity| entity.label != *text).then(|| format!("label of {field} differs"))
    })
}

/// True when `changes` set the inventory count of an item that a character
/// wears in `before`.
pub(super) fn raises_worn_count(before: &SaveView, changes: &[ViewChange]) -> bool {
    let worn: Vec<i64> = before
        .sections
        .iter()
        .flat_map(|section| &section.entities)
        .flat_map(|entity| &entity.fields)
        .filter_map(|field| match (field.id.keys(), &field.value) {
            ([Key::Name(CHARACTER), Key::Id(_), Key::Name(EQUIPMENT), Key::Id(_)], Value::Int(content)) => Some(*content),
            _ => None,
        })
        .collect();
    changes.iter().any(|change| match change {
        ViewChange::Set(id, _) => match id.keys() {
            [Key::Name(INVENTORY), Key::Id(key), Key::Name(COUNT)] => worn.contains(&i64::from(*key)),
            _ => false,
        },
        _ => false,
    })
}

/// Every stored field value by id, and the entity ids of each collection in
/// order. The computed `max_hp` and `next_level` are left out: an edit of the
/// level, the experience or the gain moves them, and the document check
/// already proves the stored keys.
fn flatten(view: &SaveView) -> (Fields, Order) {
    let mut fields = Fields::new();
    let mut order = Order::new();
    for entity in view.sections.iter().flat_map(|section| &section.entities) {
        order.entry(entity.id.section()).or_default().push(entity.id.clone());
        for field in entity.fields.iter().filter(|field| !is_computed(&field.id)) {
            fields.insert(field.id.clone(), field.value.clone());
        }
    }
    (fields, order)
}

fn is_computed(id: &FieldId) -> bool {
    matches!(id.keys().last(), Some(Key::Name(names::MAX_HP | names::NEXT_LEVEL)))
}
