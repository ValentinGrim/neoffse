//! The change summary of a session.

use neoffse_data::{Language, tables_for};

use super::Session;
use crate::changes::{Change, diff};
use crate::error::Error;

impl Session {
    /// Compares the clean state with the current state.
    ///
    /// A clean session answers with an empty list and decodes nothing.
    /// Otherwise the call opens [`Session::original`] again into a second
    /// document and builds two views: one decode plus two view builds per
    /// call, with no cache. A caller that runs it on its UI thread pays that
    /// cost once per request. Pass the language the registry was built with:
    /// it picks the table names in the texts, such as jobs and equipment.
    ///
    /// The call changes no state: the history, the dirty count and the clean
    /// point stay as they were.
    ///
    /// # Errors
    ///
    /// [`Error::Format`] when the clean bytes or either view fail to open.
    pub fn changes(&mut self, language: Language) -> Result<Vec<Change>, Error> {
        if !self.is_dirty() {
            return Ok(Vec::new());
        }
        let mut clean = self.format.open(&self.original).map_err(|source| Error::Format { source })?;
        let before = clean.view().map_err(|source| Error::Format { source })?;
        let after = self.document.view().map_err(|source| Error::Format { source })?;
        Ok(diff(&before, &after, tables_for(self.game), language))
    }
}
