//! The summary of a slot: what the Home screen shows without a view.

use std::time::Duration;

use neoffse_core::{EditError, FormatError, Game, SaveSummary, Value};

use super::read::decode_error;
use crate::document::{Document, DocumentError, Scalar, Segment};
use crate::slots::slot_for_id;

/// The `target` array of `userData.corpsList`
/// (`docs/formats/pixel-remaster.md#party`).
const CORPS: [Segment<'static>; 5] = [
    Segment::Key("userData"),
    Segment::Doc,
    Segment::Key("corpsList"),
    Segment::Doc,
    Segment::Key("target"),
];

/// A key the save lacks or holds in another shape gives an empty field.
fn is_empty_field(error: &DocumentError) -> bool {
    matches!(error, DocumentError::MissingKey { .. } | DocumentError::WrongKind { .. })
}

/// What the Home screen shows of the slot of `game`: the slot, the time
/// played, the place and the party.
///
/// - `slot`: the top-level `id`, through `slot_for_id`
///   (`docs/formats/pixel-remaster.md#slot-files`).
/// - `play_time`: `userData.playTime`, seconds, as a float or an integer
///   (`docs/formats/pixel-remaster.md#counters-and-flags`).
/// - `location`: `userData.currentLocation` when it is not empty, else
///   `userData.currentArea`
///   (`docs/formats/pixel-remaster.md#counters-and-flags`).
/// - `party`: the names of `userData.corpsList`, in list order, however many
///   rows it holds (`docs/formats/pixel-remaster.md#party`). `name_of` reads
///   the name of a character from its `characterId`, in the middle of the
///   walk of the list, so the reads of the entries and of the names keep their
///   order. An entry whose `characterId` is absent, is no integer or names no
///   character (`MissingRow`) is skipped.
///
/// A key the save lacks, or holds in another shape, gives the empty value of
/// its field. The call changes no byte of the document.
///
/// # Errors
///
/// `FormatError::Decode` with the `DocumentError` or `EditError` source for
/// any other failure, such as a `userData` that is an empty string or a
/// character name that is not text.
pub(crate) fn summary<N>(document: &mut Document, game: Game, mut name_of: N) -> Result<SaveSummary, FormatError>
where
    N: FnMut(&mut Document, u32) -> Result<Value, EditError>,
{
    let mut summary = SaveSummary::new(game);
    if let Some(Scalar::Int(id)) = optional(document, &[Segment::Key("id")])? {
        summary.slot = slot_for_id(id);
    }
    // A negative or non-finite time is no play time: the field takes its empty value.
    summary.play_time = match optional(document, &user_data("playTime"))? {
        Some(Scalar::Float(seconds)) => Duration::try_from_secs_f64(seconds).ok(),
        Some(Scalar::Int(seconds)) => u64::try_from(seconds).ok().map(Duration::from_secs),
        _ => None,
    };
    for key in ["currentLocation", "currentArea"] {
        if let Some(Scalar::Text(place)) = optional(document, &user_data(key))?
            && !place.is_empty()
        {
            summary.location = Some(place);
            break;
        }
    }
    summary.party = party_names(document, &mut name_of)?;
    Ok(summary)
}

/// The scalar at `path`, `None` for an absent key or another shape.
fn optional(document: &mut Document, path: &[Segment<'_>]) -> Result<Option<Scalar>, FormatError> {
    match document.get(path) {
        Ok(scalar) => Ok(Some(scalar)),
        Err(error) if is_empty_field(&error) => Ok(None),
        Err(error) => Err(decode_error(error)),
    }
}

fn party_names<N>(document: &mut Document, name_of: &mut N) -> Result<Vec<String>, FormatError>
where
    N: FnMut(&mut Document, u32) -> Result<Value, EditError>,
{
    let len = match document.array_len(&CORPS) {
        Ok(len) => len,
        Err(error) if is_empty_field(&error) => return Ok(Vec::new()),
        Err(error) => return Err(decode_error(error)),
    };
    let mut party = Vec::new();
    for index in 0..len {
        let mut path = CORPS.to_vec();
        path.extend([Segment::Index(index), Segment::Doc, Segment::Key("characterId")]);
        let Some(Scalar::Int(id)) = optional(document, &path)? else { continue };
        let Ok(id) = u32::try_from(id) else { continue };
        match name_of(document, id) {
            Ok(Value::Text(name)) => party.push(name),
            Ok(_) | Err(EditError::MissingRow { .. }) => {}
            Err(error) => return Err(decode_error(error)),
        }
    }
    Ok(party)
}

fn user_data(key: &'static str) -> [Segment<'static>; 3] {
    [Segment::Key("userData"), Segment::Doc, Segment::Key(key)]
}
