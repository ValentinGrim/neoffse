//! A second format that refuses every file, to register a game next to FF I.

use std::sync::Arc;

use neoffse::Registry;
use neoffse::core::{Detection, FileProbe, FormatError, Game, SaveDocument, SaveFiles, SaveFormat};
use neoffse::data::Language;
use neoffse::pr::OpenError;
use neoffse::pr::ff1::Ff1Format;

/// A format of `game` that claims nothing and refuses every file as a slot of
/// `found` with `vehicles` vehicles, the error of a Pixel Remaster format
/// that identified another game.
pub(crate) struct Refusing {
    pub(crate) game: Game,
    pub(crate) found: Option<Game>,
    pub(crate) vehicles: usize,
}

impl SaveFormat for Refusing {
    fn game(&self) -> Game {
        self.game
    }

    fn detect(&self, _probe: &FileProbe<'_>) -> Option<Detection> {
        None
    }

    fn open(&self, _files: &SaveFiles) -> Result<Box<dyn SaveDocument>, FormatError> {
        let source = OpenError::OtherGame {
            found: self.found,
            vehicles: self.vehicles,
        };
        Err(FormatError::Decode { source: Box::new(source) })
    }
}

/// The FF I format alone plus `extra`, through `Registry::from_formats`.
pub(crate) fn registry_with(extra: Vec<Arc<dyn SaveFormat>>) -> Arc<Registry> {
    let mut formats: Vec<Arc<dyn SaveFormat>> = vec![Arc::new(Ff1Format::with_language(Language::En))];
    formats.extend(extra);
    Arc::new(Registry::from_formats(formats))
}
