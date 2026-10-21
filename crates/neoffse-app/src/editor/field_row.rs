//! The Slint row of one field.

use neoffse::core::{FieldMeta, FieldView, Value};
use neoffse::data::{GameTables, Language};
use slint::{ModelRc, SharedString, VecModel};

use super::kind::field_kind;
use super::pick::entry_name;
use crate::{FieldKind, FieldRow};

/// A Slint model that holds `items`.
pub(super) fn model_of<T: Clone + 'static>(items: Vec<T>) -> ModelRc<T> {
    ModelRc::new(VecModel::from(items))
}

/// What the row builder needs besides the field.
#[derive(Debug, Clone, Copy)]
pub(super) struct FieldContext {
    pub(super) tables: Option<&'static GameTables>,
    pub(super) language: Language,
    pub(super) expert: bool,
}

/// The row for `view`, under `key`, with the lock the game interface sets.
///
/// A value whose type disagrees with its shape (an integer in a string
/// field) shows as a `label` with no text: the row never panics.
pub(super) fn field_row(context: &FieldContext, key: i32, view: &FieldView, locked: bool) -> FieldRow {
    let meta = &view.meta;
    let mismatch = meta.shape.kind() != view.value.kind();
    let kind = if mismatch { FieldKind::Label } else { field_kind(meta, context.tables) };
    let mut row = FieldRow {
        key,
        label: meta.label.as_str().into(),
        kind,
        known: true,
        editable: kind != FieldKind::Label && (context.expert || !locked),
        locked,
        cap: meta.cap.map(|cap| cap.to_string()).unwrap_or_default().into(),
        above_cap: meta.exceeds_cap(&view.value),
        ..FieldRow::default()
    };
    if mismatch {
        return row;
    }
    match &view.value {
        Value::Int(number) => fill_integer(context, meta, *number, &mut row),
        Value::Bool(on) => row.checked = *on,
        Value::Text(text) => row.text = text.as_str().into(),
        _ => {}
    }
    row
}

fn fill_integer(context: &FieldContext, meta: &FieldMeta, number: i64, row: &mut FieldRow) {
    row.text = number.to_string().into();
    let found = shown_name(context, meta, number);
    match row.kind {
        FieldKind::Picker => {
            row.known = found.is_some();
            row.shown = found.unwrap_or_default().into();
        }
        FieldKind::Label => row.shown = found.unwrap_or_default().into(),
        FieldKind::Choice => {
            let index = meta.names.iter().position(|named| named.value == number);
            row.names = model_of(meta.names.iter().map(|named| SharedString::from(named.name.as_str())).collect::<Vec<_>>());
            row.name_index = index.map_or(-1, |index| i32::try_from(index).unwrap_or(-1));
            row.known = index.is_some();
        }
        _ => row.known = true,
    }
}

/// The table entry name of `number`, else its named value.
fn shown_name(context: &FieldContext, meta: &FieldMeta, number: i64) -> Option<String> {
    context
        .tables
        .and_then(|tables| entry_name(tables, &meta.tables, context.language, number))
        .or_else(|| meta.name_of(number).map(str::to_owned))
}
