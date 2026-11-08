//! Helpers that the `document_*` test crates and the private-save test
//! crates share. Each crate declares `mod common;`: Cargo does not build
//! this file as a test target.
#![allow(dead_code, reason = "each test crate uses a subset of the shared helpers")]
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

use std::ffi::OsStr;
use std::path::PathBuf;

pub(crate) mod edit_paths;
pub(crate) mod edit_proof;
mod edit_subject;
mod edit_view;
pub(crate) mod exit_cases;
pub(crate) mod exit_equip;
mod exit_facts;
pub(crate) mod exit_ff2;
mod exit_ff2_spell;
mod exit_fields;
mod exit_growth;
pub(crate) mod ff1_ability_rules;
pub(crate) mod ff1_format;
pub(crate) mod ff1_magic;
pub(crate) mod ff1_real;
pub(crate) mod ff1_render;
pub(crate) mod ff1_rules;
pub(crate) mod ff1_save;
pub(crate) mod ff2_ability_rules;
pub(crate) mod ff2_real;
pub(crate) mod ff2_render;
pub(crate) mod ff2_rules;
pub(crate) mod ff2_save;
pub(crate) mod ff2_spell;
pub(crate) mod growth_oracle;
pub(crate) mod json_diff;
mod json_scan;
pub(crate) mod json_tree;
pub(crate) mod pr_files;
pub(crate) mod rolls;
pub(crate) mod slot_model;
mod slot_rules;

use ff1_format::{one_file, sav};
use ff1_save::Row;
use neoffse_core::{CapPolicy, Edit, EditError, FieldId, GameChange, RowId, SaveFormat, SaveSlot, Value};
use neoffse_format_pr::container::Container;
use neoffse_format_pr::document::{DocumentError, Segment};
use neoffse_format_pr::shared::names;

/// The six game folders under the saves root, one per game of
/// `docs/formats/pixel-remaster.md#games`.
pub(crate) const GAMES: [&str; 6] = ["ff1", "ff2", "ff3", "ff4", "ff5", "ff6"];

/// Steam writes this file beside the saves; it is not a save
/// (`docs/formats/pixel-remaster.md#other-files-in-the-folder`).
pub(crate) const STEAM_CLOUD_MARKER: &str = "steam_autocloud.vdf";

/// True when a file of a private game folder can be a save: not the Steam
/// cloud marker, not a dot-file (KDE writes `.directory`), and named as the
/// game names its files, 44 characters of the base64 alphabet that may hold
/// `=` (`docs/formats/pixel-remaster.md#slot-files`). This leaves out stray
/// files such as `desktop.ini` and `README.txt`.
pub(crate) fn is_save_name(name: &OsStr) -> bool {
    name != STEAM_CLOUD_MARKER
        && !name.as_encoded_bytes().starts_with(b".")
        && name.len() == 44
        && name
            .as_encoded_bytes()
            .iter()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'+' | b'/' | b'='))
}

/// The private save files of `game`: the files [`is_save_name`] accepts.
/// `None` when the game has no private saves or only other files;
/// the second case reports the skip through `neoffse_testkit::skip`.
pub(crate) fn save_files(game: &str) -> Option<Vec<PathBuf>> {
    let mut files = neoffse_testkit::private_saves(game)?;
    files.retain(|path| path.file_name().is_none_or(is_save_name));
    if files.is_empty() {
        neoffse_testkit::skip(&format!("{game} holds no file named like a save"));
        return None;
    }
    Some(files)
}

/// An owned path step, so a test can build paths and borrow them as
/// [`Segment`]s through [`segments`].
#[derive(Debug, Clone)]
pub(crate) enum Step {
    Key(String),
    Index(usize),
    Doc,
}

pub(crate) fn segments(steps: &[Step]) -> Vec<Segment<'_>> {
    steps
        .iter()
        .map(|step| match step {
            Step::Key(key) => Segment::Key(key),
            Step::Index(index) => Segment::Index(*index),
            Step::Doc => Segment::Doc,
        })
        .collect()
}

/// The path to the character at `index` of the party, then `rest`. It ends
/// at the element, a string that holds a document: `rest` starts with
/// [`Segment::Doc`] to read inside it.
pub(crate) fn character_path<'a>(index: usize, rest: &[Segment<'a>]) -> Vec<Segment<'a>> {
    let mut path = vec![
        Segment::Key("userData"),
        Segment::Doc,
        Segment::Key("ownedCharacterList"),
        Segment::Doc,
        Segment::Key("target"),
        Segment::Index(index),
    ];
    path.extend_from_slice(rest);
    path
}

/// Wraps `doc` in quotes and escapes `\` and `"`, as the game embeds a document
/// (`docs/formats/pixel-remaster.md#documents-inside-strings`).
pub(crate) fn embed(doc: &str) -> String {
    embed_with(doc, false)
}

/// Like [`embed`]; with `unicode_quote` each `"` becomes `\u0022`, a form the
/// game's writer does not produce but a reader must accept: the game writes
/// only `\"` and `\\` escapes (`docs/formats/pixel-remaster.md#json-text`).
pub(crate) fn embed_with(doc: &str, unicode_quote: bool) -> String {
    let quote = if unicode_quote { "\\u0022" } else { "\\\"" };
    format!("\"{}\"", doc.replace('\\', "\\\\").replace('"', quote))
}

/// What remains of `a` and `b` once the common prefix and suffix are gone.
/// Both cuts move back to a char boundary, so multi-byte text never splits.
pub(crate) fn changed_span<'a>(a: &'a str, b: &'a str) -> (&'a str, &'a str) {
    let mut prefix = a.bytes().zip(b.bytes()).take_while(|(x, y)| x == y).count();
    while !(a.is_char_boundary(prefix) && b.is_char_boundary(prefix)) {
        prefix -= 1;
    }
    let (a, b) = (&a[prefix..], &b[prefix..]);
    let mut suffix = a.bytes().rev().zip(b.bytes().rev()).take_while(|(x, y)| x == y).count();
    while !(a.is_char_boundary(a.len() - suffix) && b.is_char_boundary(b.len() - suffix)) {
        suffix -= 1;
    }
    (&a[..a.len() - suffix], &b[..b.len() - suffix])
}

/// `Enforce` and `StorageOnly` as the raw-interface tests name them.
pub(crate) const ENFORCE: CapPolicy = CapPolicy::Enforce;
pub(crate) const STORAGE_ONLY: CapPolicy = CapPolicy::StorageOnly;

/// The storage bound of a number field: `i32::MAX`
/// (`docs/formats/pixel-remaster.md#bounds-in-the-master-data`).
pub(crate) const STORAGE_MAX: i64 = 2_147_483_647;

/// The id of a field of a character.
pub(crate) fn character_field(id: u32, name: &'static str) -> FieldId {
    FieldId::new(names::CHARACTER).with_id(id).with_name(name)
}

/// The character `id` as an entity.
pub(crate) fn character(id: u32) -> FieldId {
    FieldId::new(names::CHARACTER).with_id(id)
}

/// The row `key` of the item list `list`.
pub(crate) fn item(list: &'static str, key: u32) -> FieldId {
    FieldId::new(list).with_id(key)
}

/// The equipment slot `slot` of character `id`.
pub(crate) fn slot(id: u32, slot: u32) -> FieldId {
    character_field(id, names::EQUIPMENT).with_id(slot)
}

/// The `count` of that slot.
pub(crate) fn slot_count(id: u32, slot_number: u32) -> FieldId {
    slot(id, slot_number).with_name(names::COUNT)
}

/// The `count` of the row `key` of `list`.
pub(crate) fn count(list: &'static str, key: u32) -> FieldId {
    item(list, key).with_name(names::COUNT)
}

pub(crate) fn row(list: &'static str, key: u32) -> RowId {
    RowId::new(FieldId::new(list), key)
}

pub(crate) fn gil() -> FieldId {
    FieldId::new(names::GIL)
}

/// Sets slot `slot_number` of character `id` to the content `content`.
pub(crate) fn equip(id: u32, slot_number: u32, content: u32) -> Edit {
    Edit::set(slot(id, slot_number), Value::Int(i64::from(content)))
}

/// The forward and inverse edits of `change`.
pub(crate) fn plain(change: &GameChange) -> (&[Edit], &[Edit]) {
    (change.forward.edits(), change.inverse.edits())
}

/// Sets the count of the row `key` of `rows` to `new_count`.
pub(crate) fn with_count(rows: &mut [Row], key: u32, new_count: u32) {
    let text = format!("{{\"contentId\":{key},\"count\":");
    let index = rows.iter().position(|row| row.text.starts_with(&text)).unwrap();
    rows[index] = Row::new(key, new_count);
}

/// Drops the five placeholder rows that open the standard inventory.
pub(crate) fn without_placeholders(rows: &mut Vec<Row>) {
    rows.drain(..5);
}

/// The document error under the `Format` error of `result`.
pub(crate) fn assert_document_error(result: Result<Value, EditError>) -> DocumentError {
    let Err(EditError::Format { source, .. }) = result else {
        panic!("no Format error")
    };
    *source.downcast::<DocumentError>().expect("the source is a DocumentError")
}

/// `userData.playTime`, `userData.currentLocation` and the corps list of
/// the user data.
pub(crate) const PLAY_TIME: [Segment<'static>; 3] = [Segment::Key("userData"), Segment::Doc, Segment::Key("playTime")];
pub(crate) const LOCATION: [Segment<'static>; 3] = [Segment::Key("userData"), Segment::Doc, Segment::Key("currentLocation")];
pub(crate) const CORPS: [Segment<'static>; 5] = [
    Segment::Key("userData"),
    Segment::Doc,
    Segment::Key("corpsList"),
    Segment::Doc,
    Segment::Key("target"),
];

/// The path to the `characterId` of the corps row `index`.
pub(crate) fn corps_id(index: usize) -> [Segment<'static>; 8] {
    let [user, doc, list, list_doc, target] = CORPS;
    [
        user,
        doc,
        list,
        list_doc,
        target,
        Segment::Index(index),
        Segment::Doc,
        Segment::Key("characterId"),
    ]
}

/// The list of vehicles in the user data.
pub(crate) const VEHICLES: [Segment<'static>; 5] = [
    Segment::Key("userData"),
    Segment::Doc,
    Segment::Key("ownedTransportationList"),
    Segment::Doc,
    Segment::Key("target"),
];

/// One vehicle row as the synthetic slots write it.
pub(crate) const VEHICLE: &str = r#"{"position":{"x":1000.0,"y":1000.0,"z":-1000.0},"direction":0,"id":1,"mapId":-1,"enable":false,"timeStampTicks":0}"#;

/// The `id` pair that opens a synthetic slot text.
const ID_PAIR: &str = "\"id\":1,";

/// The slot `text` with its `id` pair removed, or set to 99.
pub(crate) fn id_variants(text: &str) -> [(&'static str, String); 2] {
    assert!(text.starts_with(&format!("{{{ID_PAIR}")), "the id is the first pair");
    [
        ("absent", text.replacen(ID_PAIR, "", 1)),
        ("out of range", text.replacen(ID_PAIR, "\"id\":99,", 1)),
    ]
}

/// The slot that `format` reports for `text` stored under the file name `name`.
pub(crate) fn slot_under(format: &impl SaveFormat, name: &str, text: &str) -> Option<SaveSlot> {
    let files = one_file(name, sav(text, &Container::game()));
    format.open(&files).unwrap().summary().unwrap().slot
}
