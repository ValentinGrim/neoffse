//! The summary of a slot: what the Home screen shows without a view.

use neoffse_core::{FieldId, FormatError, SaveSummary};

use super::binding::Part;
use super::{Ff2Model, GAME, names};
use crate::shared::access;
use crate::shared::summary::summary;

impl Ff2Model {
    /// What the Home screen shows of the slot: the slot, the time played, the
    /// place and the party.
    ///
    /// - `slot`: the top-level `id`, through `slot_for_id`
    ///   (`docs/formats/pixel-remaster.md#slot-files`).
    /// - `play_time`: `userData.playTime`, seconds, as a float or an integer
    ///   (`docs/formats/pixel-remaster.md#counters-and-flags`).
    /// - `location`: `userData.currentLocation` when it is not empty, else
    ///   `userData.currentArea`
    ///   (`docs/formats/pixel-remaster.md#counters-and-flags`).
    /// - `party`: the names of `userData.corpsList`, in list order. An empty
    ///   seat (`characterId` 0) names no character and is skipped, as is an
    ///   entry whose `characterId` is absent or is no integer
    ///   (`docs/formats/pixel-remaster.md#people-and-party-in-ff-ii`). An
    ///   empty character name stays `""` in `party`, and a `characterId`
    ///   listed twice gives the name twice.
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
        let tables = self.tables;
        summary(&mut self.document, GAME, |document, id| {
            access::get_in::<Part>(document, tables, &FieldId::new(names::CHARACTER).with_id(id).with_name(names::NAME))
        })
    }
}
