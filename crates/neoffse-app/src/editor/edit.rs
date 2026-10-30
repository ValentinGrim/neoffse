//! Edits: a commit, a new row, a removed row. Each becomes one edit on the
//! open session.

use neoffse::Session;
use neoffse::core::{Edit, EditError, RowId};

use super::input::{Input, value_for};
use super::model::{EditorViewModel, at, not_editable};
use super::notice::{notice_of, problem_notice};
use super::pick::entry_name;
use crate::{EditNotice, EditProblem};

/// The notice for a failed edit or replay, and whether the session is lost.
/// A failed rollback reloads the save from its last clean state; when that
/// fails too, the document state is unknown: the problem becomes `Internal`
/// and the flag is `true`.
pub(super) fn settle(session: &mut Session, error: &EditError, label: &str) -> (EditNotice, bool) {
    let mut notice = notice_of(error, label);
    let lost = matches!(error, EditError::RollbackFailed { .. }) && session.discard().is_err();
    if lost {
        notice.problem = EditProblem::Internal;
    }
    (notice, lost)
}

impl EditorViewModel {
    /// Sends the edit one commit means.
    ///
    /// `None`: the edit went through, or the value equals the shown one and
    /// nothing was sent. `Some`: the notice to show. A key outside the table
    /// of the last [`data`](Self::data) call, or a call with no save open,
    /// gives a `not-editable` notice, as does a running save. A session lost
    /// to a failed rollback and a failed reload closes and answers
    /// `internal`. The caller refreshes with `data`.
    #[must_use]
    #[allow(
        clippy::needless_pass_by_value,
        reason = "the glue builds the input at the call and has no use for it afterwards"
    )]
    pub fn commit(&self, key: i32, input: Input) -> Option<EditNotice> {
        let Some(slot) = self.field_slot(key) else {
            return Some(not_editable());
        };
        let value = match value_for(&slot.meta, &input) {
            Ok(value) => value,
            Err(problem) => return Some(problem_notice(problem, &slot.label)),
        };
        if value == slot.value {
            return None;
        }
        self.send(&Edit::set(slot.id, value), &slot.label)
    }

    /// Adds row `entry_id` to list `list`. The row takes the collection's
    /// initial values.
    #[must_use]
    pub fn add_row(&self, list: i32, entry_id: i32) -> Option<EditNotice> {
        let collection = at(&self.layout().lists, list);
        let (Some(collection), Ok(key)) = (collection, u32::try_from(entry_id)) else {
            return Some(not_editable());
        };
        // The label only names the row in a notice; the session checks the id.
        let label = self
            .layout()
            .tables
            .and_then(|tables| entry_name(tables, &collection.key.tables, self.shared.language, i64::from(entry_id)))
            .unwrap_or_default();
        self.send(&Edit::insert(RowId::new(collection.id, key), None, Vec::new()), &label)
    }

    /// Removes row `row`.
    #[must_use]
    pub fn remove_row(&self, row: i32) -> Option<EditNotice> {
        let slot = at(&self.layout().rows, row);
        match slot {
            Some(slot) => self.send(&Edit::remove(slot.row), &slot.label),
            None => Some(not_editable()),
        }
    }

    /// Applies `edit` through the interface the mode picks.
    fn send(&self, edit: &Edit, label: &str) -> Option<EditNotice> {
        let expert = self.expert();
        self.on_session(label, |session| if expert { session.edit_expert(edit) } else { session.edit(edit) })
    }
}
