//! From scan data to the Slint rows.

use neoffse::core::{Game, SaveSummary};
use slint::{ModelRc, VecModel};

use super::scan::{slot_kind_of, summary_data};
use super::{FolderData, GameData, HomeData, SlotData, SummaryData};
use crate::{FolderRow, GameRow, SlotKind, SlotRow, SummaryRow};

/// The rows of the visible games. Builds Slint models: UI thread only.
pub(super) fn game_rows(data: &HomeData, owned_only: bool) -> Vec<GameRow> {
    data.games.iter().filter(|game| game.owned || !owned_only).map(game_row).collect()
}

fn game_row(game: &GameData) -> GameRow {
    let title = title_of(game.game);
    let no_save = game.supported && game.folders.iter().all(|folder| folder.listed && folder.slots.is_empty());
    // A single folder shows its slots on the game row, so its flag sits there too.
    let (slots, folders, unlisted) = match game.folders.as_slice() {
        [only] => (slot_rows(&only.slots, &title), Vec::new(), !only.listed),
        many => (Vec::new(), folder_rows(many, &title), false),
    };
    GameRow {
        id: game.game.id().into(),
        title: title.into(),
        supported: game.supported,
        slots: model(slots),
        folders: model(folders),
        no_save,
        unlisted,
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
            unlisted: !folder.listed,
        })
        .collect()
}

fn slot_rows(slots: &[SlotData], title: &str) -> Vec<SlotRow> {
    slots.iter().map(|slot| slot_row(slot, title)).collect()
}

fn slot_row(slot: &SlotData, title: &str) -> SlotRow {
    SlotRow {
        key: slot.key,
        summary: summary_row(title, slot.kind, slot.number, slot.summary.as_ref()),
        summarized: slot.summary.is_some(),
        unreadable: slot.unreadable,
        openable: slot.openable,
    }
}

fn summary_row(title: &str, kind: SlotKind, number: i32, found: Option<&SummaryData>) -> SummaryRow {
    let mut summary = SummaryRow {
        game: title.into(),
        kind,
        number,
        ..SummaryRow::default()
    };
    if let Some(found) = found {
        summary.party = found.party.as_str().into();
        summary.location = found.location.as_str().into();
        summary.play_time = found.play_time.as_str().into();
    }
    summary
}

/// The row the editor page shows for an open save: the same mapping as a
/// Home slot with a summary. `name` is the file name, the fallback of the
/// slot rule.
pub(super) fn summary_row_of(summary: &SaveSummary, name: &str) -> SummaryRow {
    let (kind, number) = slot_kind_of(Some(summary), name);
    summary_row(&title_of(summary.game), kind, number, Some(&summary_data(summary)))
}

/// `Final Fantasy I` to `Final Fantasy VI`; any other number prints as digits.
pub(crate) fn title_of(game: Game) -> String {
    let number = game.number();
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
