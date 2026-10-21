//! Reads that find a row or a dictionary entry and the path that reaches it.
//!
//! Rows are found by the value of `id` or `contentId`, dictionary entries by
//! their key, never by position (`docs/formats/pixel-remaster.md#characters`,
//! pitfall 22). Every lookup is a read: it parses levels and never marks one
//! dirty.

use neoffse_core::{EditError, FieldId, RowId};

use super::binding::{Binding, Collection, Part, Path, characters_target, element, with};
use super::names;
use crate::document::{Document, DocumentError, Scalar, Segment, render_path};

/// The error of a failed document read, bound to the field that asked.
pub(super) fn format_error(field: &FieldId, source: DocumentError) -> EditError {
    EditError::Format {
        field: field.clone(),
        source: Box::new(source),
    }
}

/// The scalar at `path` as an integer. A float, a text or a `Uint` above
/// `i64::MAX` is `WrongKind { expected: "integer" }`.
pub(super) fn read_int(document: &mut Document, path: &[Segment<'_>]) -> Result<i64, DocumentError> {
    match document.get(path)? {
        Scalar::Int(int) => Ok(int),
        Scalar::Uint(uint) => i64::try_from(uint).map_err(|_| wrong_kind(path, "integer")),
        _ => Err(wrong_kind(path, "integer")),
    }
}

/// The scalar at `path` as text; any other kind is `WrongKind { expected: "string" }`.
pub(super) fn read_text(document: &mut Document, path: &[Segment<'_>]) -> Result<String, DocumentError> {
    match document.get(path)? {
        Scalar::Text(text) => Ok(text),
        _ => Err(wrong_kind(path, "string")),
    }
}

fn wrong_kind(path: &[Segment<'_>], expected: &'static str) -> DocumentError {
    DocumentError::WrongKind {
        path: render_path(path),
        expected,
    }
}

/// The identifier of the row `key` (a `u32` by construction of the caller),
/// or a `WrongKind` when the stored value is no `u32`.
fn read_key(document: &mut Document, path: &[Segment<'_>]) -> Result<u32, DocumentError> {
    u32::try_from(read_int(document, path)?).map_err(|_| wrong_kind(path, "unsigned 32-bit integer"))
}

/// The `id` or `contentId` of each element of `collection`, in document order.
pub(super) fn row_keys(document: &mut Document, collection: Collection) -> Result<Vec<u32>, DocumentError> {
    let target = collection.target();
    let len = document.array_len(&target)?;
    (0..len)
        .map(|index| {
            let key = Segment::Key(collection.row_key());
            read_key(document, &with(element(target.clone(), index), &[key]))
        })
        .collect()
}

/// Index of the first element of `collection` whose key is `wanted`. Keys are
/// read in order up to the match: an element whose key is no integer, before
/// the match, fails the call with `WrongKind` instead of being skipped.
pub(super) fn find_row(document: &mut Document, collection: Collection, wanted: u32) -> Result<Option<usize>, DocumentError> {
    let target = collection.target();
    for index in 0..document.array_len(&target)? {
        let key = with(element(target.clone(), index), &[Segment::Key(collection.row_key())]);
        if read_int(document, &key)? == i64::from(wanted) {
            return Ok(Some(index));
        }
    }
    Ok(None)
}

/// Index of the entry whose key is `wanted` in the dictionary at `dictionary`
/// (`docs/formats/pixel-remaster.md#container-shapes`).
fn find_entry(document: &mut Document, dictionary: &[Segment<'static>], wanted: u32) -> Result<Option<usize>, DocumentError> {
    let keys = with(dictionary.to_vec(), &[Segment::Key("keys")]);
    for index in 0..document.array_len(&keys)? {
        if read_int(document, &with(keys.clone(), &[Segment::Index(index)]))? == i64::from(wanted) {
            return Ok(Some(index));
        }
    }
    Ok(None)
}

fn missing_row(collection: &'static str, key: u32) -> EditError {
    EditError::MissingRow {
        row: RowId::new(FieldId::new(collection), key).id().clone(),
    }
}

/// The path to the value of `binding`. `MissingRow` for an absent row,
/// `UnknownField` for a dictionary without the key, `Format` for a read that
/// fails.
pub(super) fn locate(document: &mut Document, binding: &Binding, field: &FieldId) -> Result<Path, EditError> {
    let failed = |source| format_error(field, source);
    match *binding {
        Binding::Gil => Ok(vec![Segment::Key("userData"), Segment::Doc, Segment::Key("owendGil")]),
        Binding::Count { list, key } => {
            let index = find_row(document, Collection::Items(list), key)
                .map_err(failed)?
                .ok_or_else(|| missing_row(list.section(), key))?;
            Ok(with(element(list.target(), index), &[Segment::Key("count")]))
        }
        Binding::Character { id, part } => {
            let index = find_row(document, Collection::Character, id)
                .map_err(failed)?
                .ok_or_else(|| missing_row(names::CHARACTER, id))?;
            let base = element(characters_target(), index);
            locate_part(document, base, part).map_err(|error| match error {
                Located::Absent => EditError::UnknownField { field: field.clone() },
                Located::Failed(source) => format_error(field, source),
            })
        }
    }
}

enum Located {
    /// The dictionary holds no entry for the key.
    Absent,
    Failed(DocumentError),
}

impl From<DocumentError> for Located {
    fn from(source: DocumentError) -> Self {
        Self::Failed(source)
    }
}

/// An entry of a dictionary with the given key, at `values[index]`.
fn entry_value(document: &mut Document, dictionary: Path, key: u32) -> Result<Path, Located> {
    let index = find_entry(document, &dictionary, key)?.ok_or(Located::Absent)?;
    Ok(with(dictionary, &[Segment::Key("values"), Segment::Index(index)]))
}

fn locate_part(document: &mut Document, base: Path, part: Part) -> Result<Path, Located> {
    let parameter = [Segment::Key("parameter"), Segment::Doc];
    let key = |name| [Segment::Key(name)];
    Ok(match part {
        Part::Name => with(base, &key("name")),
        Part::Job => with(base, &key("jobId")),
        Part::Exp => with(base, &key("currentExp")),
        Part::Level => with(with(base, &parameter), &key("addtionalLevel")),
        Part::Hp => with(with(base, &parameter), &key("currentHP")),
        Part::MaxHpGain => with(with(base, &parameter), &key("addtionalMaxHp")),
        Part::Stat(stat) => with(with(base, &parameter), &key(stat.document_key())),
        Part::Charges(level) | Part::ChargesMaxGain(level) => {
            let list = if matches!(part, Part::Charges(_)) {
                "currentMpCountList"
            } else {
                "addtionalMaxMpCountList"
            };
            let dictionary = with(with(base, &parameter), &[Segment::Key(list), Segment::Doc]);
            entry_value(document, dictionary, level)?
        }
        Part::Slot(slot) | Part::SlotCount(slot) => {
            let dictionary = with(base, &[Segment::Key("equipmentList"), Segment::Doc]);
            let value = entry_value(document, dictionary, slot)?;
            let name = if matches!(part, Part::Slot(_)) { "contentId" } else { "count" };
            with(value, &[Segment::Doc, Segment::Key(name)])
        }
    })
}

/// The number of equipment slots, over all characters, that hold
/// `content_id` (`docs/formats/pixel-remaster.md#inventory`: an equipped id
/// appears in the inventory with a count at least this number).
pub(super) fn wearers(document: &mut Document, content_id: u32) -> Result<usize, DocumentError> {
    let target = characters_target();
    let mut count = 0;
    for index in 0..document.array_len(&target)? {
        let dictionary = with(element(target.clone(), index), &[Segment::Key("equipmentList"), Segment::Doc]);
        let keys = with(dictionary.clone(), &[Segment::Key("keys")]);
        for entry in 0..document.array_len(&keys)? {
            let slot = read_key(document, &with(keys.clone(), &[Segment::Index(entry)]))?;
            if !(1..=names::EQUIPMENT_SLOTS).contains(&slot) {
                continue;
            }
            let value = [Segment::Key("values"), Segment::Index(entry), Segment::Doc, Segment::Key("contentId")];
            if read_key(document, &with(dictionary.clone(), &value))? == content_id {
                count += 1;
            }
        }
    }
    Ok(count)
}
