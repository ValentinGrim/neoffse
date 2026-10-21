//! The summary of a slot: what the Home screen shows without a view.

use std::time::Duration;

use neoffse_core::{EditError, FieldId, FormatError, SaveSummary, Value};

use super::{Ff1Model, GAME, names};
use crate::document::{DocumentError, Scalar, Segment};
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

pub(super) fn decode_error(source: impl std::error::Error + Send + Sync + 'static) -> FormatError {
    FormatError::Decode { source: Box::new(source) }
}

/// A key the save lacks or holds in another shape gives an empty field.
fn is_empty_field(error: &DocumentError) -> bool {
    matches!(error, DocumentError::MissingKey { .. } | DocumentError::WrongKind { .. })
}

impl Ff1Model {
    /// What the Home screen shows of the slot: the slot, the time played, the
    /// place and the party.
    ///
    /// - `slot`: the top-level `id`, through [`slot_for_id`]
    ///   (`docs/formats/pixel-remaster.md#slot-files`).
    /// - `play_time`: `userData.playTime`, seconds, as a float or an integer
    ///   (`docs/formats/pixel-remaster.md#counters-and-flags`).
    /// - `location`: `userData.currentLocation` when it is not empty, else
    ///   `userData.currentArea`.
    /// - `party`: the names of `userData.corpsList`, in list order. The format
    ///   document does not say the list order is the battle order. An entry
    ///   whose `characterId` is absent, is no integer or names no character is
    ///   skipped. An empty character name stays `""` in `party` (the view
    ///   shows a placeholder label instead), and a `characterId` listed twice
    ///   gives the name twice.
    ///
    /// A key the save lacks, or holds in another shape, gives the empty value
    /// of its field. The call changes no byte of the document.
    ///
    /// # Errors
    ///
    /// `FormatError::Decode` with the `DocumentError` or `EditError` source
    /// for any other failure, such as a `userData` that is an empty string or
    /// a character name that is not text.
    pub fn summary(&mut self) -> Result<SaveSummary, FormatError> {
        let mut summary = SaveSummary::new(GAME);
        if let Some(Scalar::Int(id)) = self.optional(&[Segment::Key("id")])? {
            summary.slot = slot_for_id(id);
        }
        summary.play_time = match self.optional(&user_data("playTime"))? {
            Some(Scalar::Float(seconds)) => Duration::try_from_secs_f64(seconds).ok(),
            Some(Scalar::Int(seconds)) => u64::try_from(seconds).ok().map(Duration::from_secs),
            _ => None,
        };
        for key in ["currentLocation", "currentArea"] {
            if let Some(Scalar::Text(place)) = self.optional(&user_data(key))?
                && !place.is_empty()
            {
                summary.location = Some(place);
                break;
            }
        }
        summary.party = self.party_names()?;
        Ok(summary)
    }

    /// The scalar at `path`, `None` for an absent key or another shape.
    fn optional(&mut self, path: &[Segment<'_>]) -> Result<Option<Scalar>, FormatError> {
        match self.read(path) {
            Ok(scalar) => Ok(Some(scalar)),
            Err(error) if is_empty_field(&error) => Ok(None),
            Err(error) => Err(decode_error(error)),
        }
    }

    fn party_names(&mut self) -> Result<Vec<String>, FormatError> {
        let len = match self.document.array_len(&CORPS) {
            Ok(len) => len,
            Err(error) if is_empty_field(&error) => return Ok(Vec::new()),
            Err(error) => return Err(decode_error(error)),
        };
        let mut party = Vec::new();
        for index in 0..len {
            let mut path = CORPS.to_vec();
            path.extend([Segment::Index(index), Segment::Doc, Segment::Key("characterId")]);
            let Some(Scalar::Int(id)) = self.optional(&path)? else { continue };
            let Ok(id) = u32::try_from(id) else { continue };
            let field = FieldId::new(names::CHARACTER).with_id(id).with_name(names::NAME);
            match self.get(&field) {
                Ok(Value::Text(name)) => party.push(name),
                Ok(_) | Err(EditError::MissingRow { .. }) => {}
                Err(error) => return Err(decode_error(error)),
            }
        }
        Ok(party)
    }
}

fn user_data(key: &'static str) -> [Segment<'static>; 3] {
    [Segment::Key("userData"), Segment::Doc, Segment::Key(key)]
}
