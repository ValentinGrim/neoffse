//! The open pipeline of a slot file: container, JSON text, game rule and
//! shape check, written over the model of one game.

use std::collections::HashSet;

use neoffse_core::{EditError, FieldId, FormatError, RowId, SaveFiles};
use neoffse_data::{GameTables, Language};

use super::SlotDocument;
use super::SlotModel;
use super::names;
use super::read::decode_error;
use crate::container::decode;
use crate::document::Document;
use crate::games::{check_markers, game_of_count};
use crate::open_error::OpenError;

/// Opens the one file of `files` as a slot of the game of `M`.
///
/// The steps run in this order, each failing as `FormatError::Decode`: file
/// count, container, JSON text, game of the vehicle count, refusal of
/// another game, marker keys, then the shape gate over the character, the
/// inventory and the key item lists, then a repeated key. The game rule is
/// `docs/formats/pixel-remaster.md#telling-the-games-apart`.
pub(crate) fn open_slot<M: SlotModel>(tables: &'static GameTables, language: Language, files: &SaveFiles) -> Result<SlotDocument<M>, FormatError> {
    if files.len() != 1 {
        return Err(decode_error(OpenError::FileCount { found: files.len() }));
    }
    let Some((name, bytes)) = files.iter().next() else {
        return Err(decode_error(OpenError::FileCount { found: 0 }));
    };
    let decoded = decode(bytes).map_err(decode_error)?;
    let mut document = Document::parse(&decoded.json).map_err(decode_error)?;
    let (game, vehicles) = game_of_count(&mut document).map_err(decode_error)?;
    if game != M::GAME {
        return Err(decode_error(OpenError::OtherGame { found: Some(game), vehicles }));
    }
    check_markers(&mut document, M::GAME, vehicles).map_err(decode_error)?;
    let mut model = M::new(document, tables);
    for collection in [names::CHARACTER, names::INVENTORY, names::KEY_ITEMS] {
        let not_a_slot = |source| decode_error(OpenError::NotASlot { source: Box::new(source) });
        let keys = model.rows(&FieldId::new(collection)).map_err(not_a_slot)?;
        if let Some(key) = repeated_key(&keys) {
            let row = RowId::new(FieldId::new(collection), key).id().clone();
            return Err(not_a_slot(EditError::DuplicateRow { row }));
        }
    }
    Ok(SlotDocument::new(model, decoded.container, name.to_owned(), language))
}

/// The first key that appears twice in `keys`, in document order.
fn repeated_key(keys: &[u32]) -> Option<u32> {
    let mut seen = HashSet::new();
    keys.iter().copied().find(|key| !seen.insert(*key))
}
