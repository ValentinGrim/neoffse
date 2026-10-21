//! The 22 slot files of a Pixel Remaster save folder and the detection that
//! recognizes them (`docs/formats/pixel-remaster.md#slot-files`).
//!
//! The six games share these names, so nothing here carries a game. A
//! name is the only proof of a slot: the head of a file is base64 text that
//! names no game, and the facade breaks the tie between the six formats with
//! the game of the save folder.

use neoffse_core::{Confidence, Detection, FileProbe, PROBE_HEAD_LEN, SaveSlot};

use crate::container::text::split;

/// One slot file of the game: the number the JSON `id` key holds and the PC
/// file name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct SlotFile {
    /// The value of the top-level `id` key: 1 to 20 for a manual slot, 21 for
    /// the Auto Save, 22 for the Quick Save.
    pub id: u32,
    /// The PC file name, 44 characters of the base64 alphabet.
    pub name: &'static str,
}

const fn slot(id: u32, name: &'static str) -> SlotFile {
    SlotFile { id, name }
}

/// The 22 slot files in the order of the format document: Auto Save (21),
/// Quick Save (22), then slots 1 to 20.
///
/// Source: `docs/formats/pixel-remaster.md#slot-files`. A test pins every row
/// to that table.
pub const SLOT_FILES: [SlotFile; 22] = [
    slot(21, "7nCxyzTwG31W3Zlg70mo751W8ETH1n+Km0dWOzRU84Y="),
    slot(22, "Rl18osV3e9kPX9SMWQj8mqShFpTUmu1lf6Mb=FVVfqk="),
    slot(1, "ookrbATYovG3tEOXIH4HqWnsv8TrUlRWzM8AlCmW2mk="),
    slot(2, "vgU2wnuaPje2Or53Iqs8Mp=Al6sdM+GM04Iymv229Ow="),
    slot(3, "uhHNR4g5QL5twqCc+IhexaltjtBjJnzzcxh5RBSy4G4="),
    slot(4, "fmsBRQ+D6YzdjCbBbl7BQuagHyg=7iX3I=EnhccyGDM="),
    slot(5, "NXa+MQ+hiHKlPAHJ6GiVWi2Wk5JR2xQQaQxzhyCbK2E="),
    slot(6, "UWtRedIOaeA6ig=8r6DIvxg33X92oMM9P8JBwiag4d0="),
    slot(7, "e1gfNt2iCE2I3yucQ8zfXn0ou+P2=lREb2q7Lqm04Gc="),
    slot(8, "6Pf6Ky7e4QBPuKH9EFJ1Iu+BUEz0zNrXdaS8866Gcq0="),
    slot(9, "9dHjN5+9JJWfJ9xoprXo=ehwoEwJwKRYL1Hlc92UNQk="),
    slot(10, "oY6N7KlcC4jscZnfa4ea6Nr=TUSR+I=29kwPNZe2NAo="),
    slot(11, "NKQ3ux2pea=DqE=vXPKb8+oix5Lt467opYaG0p0brgU="),
    slot(12, "HyhjsKWa=tCVf3TWB3qRy7NyrJbc8orciJCntDpqT=I="),
    slot(13, "hl9YCUf633k79xePC9PiKAEOq1ajUcSZkLofQuNw2OM="),
    slot(14, "C=ozNkSxgKEoLCgOPLJakAUUhnL820LbGlpMz0irQFI="),
    slot(15, "z2837SldCS+oIV8y4w5LrnJK9URKYy1QrnoA9bvCg5o="),
    slot(16, "CnvUyfaDeqDg3XbVpVWJOj=sPKcGMCV3dR=xM8Ze5jE="),
    slot(17, "eQ9Km3NT1WoE4h0hFD90ggFIZayYxfHkIVntc7akYVo="),
    slot(18, "Lnbq+GaFOc4ybPZaCf=llI0arXo06rJL32Eu+mCwsLg="),
    slot(19, "9GkO1xc52WAzswcEtJxs963MkuCohOHgYj0Fhio=fPE="),
    slot(20, "mkYfUr4Mtg0zUmF=6lw+bxRLnbnBYp9ayg1KgploDpQ="),
];

/// The slot a JSON `id` value names: 1 to 20 are manual slots, 21 is the
/// Auto Save, 22 the Quick Save (`docs/formats/pixel-remaster.md#slot-files`).
/// Any other value gives `None`.
#[must_use]
pub fn slot_for_id(id: i64) -> Option<SaveSlot> {
    match id {
        21 => Some(SaveSlot::Auto),
        22 => Some(SaveSlot::Quick),
        1..=20 => u32::try_from(id).ok().map(SaveSlot::Numbered),
        _ => None,
    }
}

/// The slot a file name stands for, by exact byte match against
/// [`SLOT_FILES`]: case, spaces and any directory part count.
#[must_use]
pub fn slot_for_name(name: &str) -> Option<SaveSlot> {
    SLOT_FILES
        .iter()
        .find(|file| file.name == name)
        .and_then(|file| slot_for_id(i64::from(file.id)))
}

/// Claims a file when its name is one of the 22 slot names.
///
/// The name decides alone. A name outside [`SLOT_FILES`] gives `None`
/// whatever the head holds: the temporary file of the atomic write
/// (`docs/architecture.md#save-safety`), the bestiary, the slot index, the
/// system settings, the Steam marker and the `slotN.sav` names of the
/// console variant (`docs/formats/pixel-remaster.md#other-files-in-the-folder`,
/// `docs/formats/pixel-remaster.md#playstation-naming`) are all refused.
///
/// A slot name gives [`Confidence::Likely`] when the head looks like a
/// container and [`Confidence::Possible`] otherwise. It never gives
/// [`Confidence::Certain`]: at most 256 bytes of base64 text name no game,
/// and proving a slot would take a partial inflate. The six Pixel Remaster
/// formats return the same claim for the same file, and the facade breaks
/// the tie with the game of the save folder. [`Detection::slot`] comes from
/// the name.
///
/// The head looks like a container when, after an optional BOM, it holds
/// a non-empty run of base64 characters (`A-Z a-z 0-9 + / =`) up to the
/// trailing whitespace
/// (`docs/formats/pixel-remaster.md#bom-and-line-end`,
/// `docs/formats/pixel-remaster.md#base64`). The check reads at most
/// [`PROBE_HEAD_LEN`] bytes, even when a caller widened the public `head`
/// field.
#[must_use]
pub fn detect(probe: &FileProbe<'_>) -> Option<Detection> {
    let slot = slot_for_name(probe.name)?;
    let confidence = if looks_like_container(probe.head.get(..PROBE_HEAD_LEN).unwrap_or(probe.head)) {
        Confidence::Likely
    } else {
        Confidence::Possible
    };
    let mut detection = Detection::new(confidence);
    detection.slot = Some(slot);
    Some(detection)
}

fn looks_like_container(head: &[u8]) -> bool {
    let text = split(head).text;
    !text.is_empty() && text.iter().all(|&byte| byte.is_ascii_alphanumeric() || matches!(byte, b'+' | b'/' | b'='))
}
