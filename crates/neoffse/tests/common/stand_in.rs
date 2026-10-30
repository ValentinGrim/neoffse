//! A `SaveFormat` that answers on command, for the registry mechanics that
//! the real formats cannot show: two formats with different confidences, or
//! two formats that both open one file.

use std::sync::Arc;

use neoffse::Registry;
use neoffse::core::{
    CapPolicy, Capability, Confidence, Detection, Edit, EditError, FileProbe, FormatError, Game, SaveDocument, SaveFiles, SaveFormat, SaveSummary, SaveView,
};
use neoffse::data::Language;
use neoffse::pr::OpenError;
use neoffse::pr::ff1::Ff1Format;

/// Why a stand-in refuses a file.
#[derive(Clone, Copy)]
pub(crate) enum Refusal {
    /// A `Decode` error whose source is no `OpenError`.
    Garbage,
    /// A `MissingFile` error.
    MissingFile,
    /// A `Decode` error whose source is `OpenError::OtherGame`, with no vehicles.
    OtherGame(Option<Game>),
}

/// A format for one game. It declines every name and opens every file until
/// told otherwise.
pub(crate) struct StandIn {
    game: Game,
    claim: Option<Confidence>,
    refusal: Option<Refusal>,
}

impl StandIn {
    pub(crate) fn new(game: Game) -> Self {
        Self {
            game,
            claim: None,
            refusal: None,
        }
    }

    /// Claims every name, the temp names included, with `confidence`.
    pub(crate) fn claiming(mut self, confidence: Confidence) -> Self {
        self.claim = Some(confidence);
        self
    }

    /// Refuses every file with `refusal`.
    pub(crate) fn refusing(mut self, refusal: Refusal) -> Self {
        self.refusal = Some(refusal);
        self
    }

    pub(crate) fn arc(self) -> Arc<dyn SaveFormat> {
        Arc::new(self)
    }
}

impl SaveFormat for StandIn {
    fn game(&self) -> Game {
        self.game
    }

    fn detect(&self, _probe: &FileProbe<'_>) -> Option<Detection> {
        self.claim.map(Detection::new)
    }

    fn open(&self, _files: &SaveFiles) -> Result<Box<dyn SaveDocument>, FormatError> {
        match self.refusal {
            None => Ok(Box::new(StubDocument { game: self.game })),
            Some(Refusal::Garbage) => Err(FormatError::Decode {
                source: Box::new(std::io::Error::other("stand-in refusal")),
            }),
            Some(Refusal::MissingFile) => Err(FormatError::MissingFile { name: "stand-in".to_owned() }),
            Some(Refusal::OtherGame(found)) => Err(FormatError::Decode {
                source: Box::new(OpenError::OtherGame { found, vehicles: 0 }),
            }),
        }
    }
}

/// The document a stand-in opens: no section, no field, no file.
struct StubDocument {
    game: Game,
}

impl SaveDocument for StubDocument {
    fn game(&self) -> Game {
        self.game
    }

    fn capabilities(&self) -> &[Capability] {
        &[]
    }

    fn summary(&mut self) -> Result<SaveSummary, FormatError> {
        Ok(SaveSummary::new(self.game))
    }

    fn view(&mut self) -> Result<SaveView, FormatError> {
        Ok(SaveView::new(Vec::new()))
    }

    fn apply(&mut self, edit: &Edit, _policy: CapPolicy) -> Result<Edit, EditError> {
        Err(EditError::UnknownField { field: edit.target().clone() })
    }

    fn encode(&self) -> Result<SaveFiles, FormatError> {
        Ok(SaveFiles::new())
    }

    fn verify(&self, _files: &SaveFiles) -> Result<(), FormatError> {
        Ok(())
    }
}

/// A registry of FF I alone, the real format: the tests that mean "no other
/// game is registered" keep their meaning when a game joins `Registry::new`.
pub(crate) fn ff1_registry() -> Registry {
    Registry::from_formats(vec![Arc::new(Ff1Format::with_language(Language::En))])
}
