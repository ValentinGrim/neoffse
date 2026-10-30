//! The widget a field gets.

use neoffse::core::{FieldMeta, Shape};
use neoffse::data::GameTables;

use crate::FieldKind;

/// The widget a field gets, from its metadata alone.
///
/// `tables` is the game's tables, `None` for a game without any. The first
/// matching rule wins (`docs/architecture.md#principles`, rule 3):
///
/// 1. a read-only field is a `label`, whatever else the meta says;
/// 2. a flag is a `toggle`;
/// 3. a string is a `text`;
/// 4. an integer that references a table the game holds is a `picker`;
/// 5. an integer with named values is a `choice`;
/// 6. any other integer is a `number`.
///
/// A table counts only when `tables` holds it and it has an entry: a kind no
/// game fills today (`Map`) gives no picker. A shape this version does not
/// know falls to `label`.
#[must_use]
pub fn field_kind(meta: &FieldMeta, tables: Option<&GameTables>) -> FieldKind {
    if meta.read_only {
        return FieldKind::Label;
    }
    match meta.shape {
        Shape::Bool => FieldKind::Toggle,
        Shape::Text => FieldKind::Text,
        Shape::Int { .. } => {
            let has_table = tables.is_some_and(|tables| meta.tables.iter().any(|kind| tables.table(*kind).is_some_and(|entries| !entries.is_empty())));
            if has_table {
                FieldKind::Picker
            } else if !meta.names.is_empty() {
                FieldKind::Choice
            } else {
                FieldKind::Number
            }
        }
        _ => FieldKind::Label,
    }
}
