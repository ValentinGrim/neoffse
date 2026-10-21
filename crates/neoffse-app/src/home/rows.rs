//! From scan data to the Slint rows.

use slint::{ModelRc, VecModel};

use super::{FolderData, GameData, HomeData, SlotData};
use crate::{FolderRow, GameRow, SlotRow, SummaryRow};

/// The rows of the visible games. Builds Slint models: UI thread only.
pub(super) fn game_rows(data: &HomeData, owned_only: bool) -> Vec<GameRow> {
    data.games.iter().filter(|game| game.owned || !owned_only).map(game_row).collect()
}

fn game_row(game: &GameData) -> GameRow {
    let title = title_of(game);
    let no_save = game.supported && game.folders.iter().all(|folder| folder.slots.is_empty());
    let (slots, folders) = match game.folders.as_slice() {
        [only] => (slot_rows(&only.slots, &title), Vec::new()),
        many => (Vec::new(), folder_rows(many, &title)),
    };
    GameRow {
        title: title.into(),
        supported: game.supported,
        slots: model(slots),
        folders: model(folders),
        no_save,
    }
}

fn folder_rows(folders: &[FolderData], title: &str) -> Vec<FolderRow> {
    folders
        .iter()
        .enumerate()
        .map(|(index, folder)| FolderRow {
            origin: folder.origin,
            account: i32::try_from(index + 1).unwrap_or(i32::MAX),
            slots: model(slot_rows(&folder.slots, title)),
        })
        .collect()
}

fn slot_rows(slots: &[SlotData], title: &str) -> Vec<SlotRow> {
    slots.iter().map(|slot| slot_row(slot, title)).collect()
}

fn slot_row(slot: &SlotData, title: &str) -> SlotRow {
    let mut summary = SummaryRow {
        game: title.into(),
        kind: slot.kind,
        number: slot.number,
        ..SummaryRow::default()
    };
    if let Some(found) = &slot.summary {
        summary.party = found.party.as_str().into();
        summary.location = found.location.as_str().into();
        summary.play_time = found.play_time.as_str().into();
    }
    SlotRow {
        key: slot.key,
        summary,
        summarized: slot.summary.is_some(),
        unreadable: slot.unreadable,
        openable: slot.openable,
    }
}

/// `Final Fantasy I` to `Final Fantasy VI`; any other number prints as digits.
fn title_of(game: &GameData) -> String {
    let number = game.game.number();
    let numeral = match number {
        1 => "I",
        2 => "II",
        3 => "III",
        4 => "IV",
        5 => "V",
        6 => "VI",
        other => return format!("Final Fantasy {other}"),
    };
    format!("Final Fantasy {numeral}")
}

fn model<T: Clone + 'static>(rows: Vec<T>) -> ModelRc<T> {
    ModelRc::new(VecModel::from(rows))
}
