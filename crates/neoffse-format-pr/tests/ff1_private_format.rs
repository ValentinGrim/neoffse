//! `Ff1Format` and `Ff1Document` on the private save files. The folder is
//! read-only input and the tests write nothing to disk. A failure names the
//! label and a field path, never a value, a length or a name: the files hold
//! personal data.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use common::ff1_format::one_file;
use common::ff1_real::{files, slots};
use common::{GAMES, save_files};
use neoffse_core::{CapPolicy, Confidence, Edit, FieldId, FileProbe, FormatError, SaveDocument, Value};
use neoffse_format_pr::container::decode;
use neoffse_format_pr::document::{Document, Segment};
use neoffse_format_pr::ff1::{Ff1Format, OpenError, names};
use neoffse_format_pr::games::identify;
use neoffse_format_pr::slots::slot_for_name;

fn gil() -> FieldId {
    FieldId::new(names::GIL)
}

fn holds_user_data(json: &str) -> bool {
    Document::parse(json).unwrap().raw(&[Segment::Key("userData")]).is_ok()
}

#[allow(clippy::print_stderr, reason = "the counts of the run are the only output a skipped-or-not test has")]
fn report(line: &str) {
    eprintln!("{line}");
}

#[test]
fn real_slot_files_open_and_round_trip() {
    let Some(slots) = slots() else { return };
    for slot in slots {
        let files = one_file(&slot.name, slot.bytes.clone());
        let Ok(document) = Ff1Format::new().open_document(&files) else {
            panic!("{}: the file does not open", slot.label)
        };
        let original = decode(&slot.bytes).unwrap();
        let encoded = document.encode().unwrap();
        let again = decode(encoded.get(&slot.name).unwrap()).unwrap();
        assert!(again.json == original.json, "{}: the JSON text changed", slot.label);
        assert!(again.container == original.container, "{}: the container changed", slot.label);
        assert!(document.verify(&encoded).is_ok(), "{}: verify refuses the encoding", slot.label);
        assert!(document.verify(&files).is_ok(), "{}: verify refuses the original", slot.label);
    }
}

#[test]
fn real_detection_agrees_with_the_files() {
    let Some(files) = files() else { return };
    let format = Ff1Format::new();
    for file in files {
        let json = decode(&file.bytes).unwrap().json;
        let probe = FileProbe::new(&file.name, file.bytes.len() as u64, &file.bytes);
        let detection = neoffse_core::SaveFormat::detect(&format, &probe);
        if holds_user_data(&json) {
            let Some(detection) = detection else {
                panic!("{}: a slot is not detected", file.label)
            };
            assert_eq!(detection.confidence, Confidence::Likely, "{}", file.label);
            let mut document = format.open_document(&one_file(&file.name, file.bytes.clone())).unwrap();
            assert_eq!(
                detection.slot,
                document.summary().unwrap().slot,
                "{}: detection and summary disagree",
                file.label
            );
            assert_eq!(detection.slot, slot_for_name(&file.name), "{}", file.label);
        } else {
            assert!(detection.is_none(), "{}: a non-slot file is detected", file.label);
        }
    }
}

#[test]
fn real_non_slot_files_do_not_open() {
    let Some(files) = files() else { return };
    for file in files {
        if holds_user_data(&decode(&file.bytes).unwrap().json) {
            continue;
        }
        let result = Ff1Format::new().open_document(&one_file(&file.name, file.bytes.clone()));
        let Err(FormatError::Decode { source }) = result else {
            panic!("{}: a non-slot file opens", file.label)
        };
        assert!(matches!(source.downcast_ref::<OpenError>(), Some(OpenError::NotASlot { .. })), "{}", file.label);
    }
}

#[test]
fn real_apply_then_inverse_restores_the_text() {
    let Some(slots) = slots() else { return };
    for slot in slots {
        let mut document = Ff1Format::new().open_document(&one_file(&slot.name, slot.bytes.clone())).unwrap();
        let view = document.view().unwrap();
        let Some((_, field)) = view.find(&gil()) else {
            continue;
        };
        let Value::Int(old) = field.value else {
            panic!("{}: gil is no integer", slot.label)
        };
        let edit = Edit::set(gil(), Value::Int(if old == 0 { 1 } else { old - 1 }));
        let Ok(inverse) = document.apply(&edit, CapPolicy::Enforce) else {
            panic!("{}: the edit is refused", slot.label)
        };
        assert!(
            document.apply(&inverse, CapPolicy::StorageOnly).is_ok(),
            "{}: the inverse is refused",
            slot.label
        );
        let text = decode(document.encode().unwrap().get(&slot.name).unwrap()).unwrap().json;
        assert!(text == slot.json, "{}: the text changed after the inverse", slot.label);
    }
}

#[test]
fn real_verify_refuses_another_slot() {
    let Some(slots) = slots() else { return };
    let Some((first, second)) = slots.iter().find_map(|a| slots.iter().find(|b| b.json != a.json).map(|b| (a, b))) else {
        neoffse_testkit::skip("ff1 holds no two slots with different texts");
        return;
    };
    let document = Ff1Format::new().open_document(&one_file(&first.name, first.bytes.clone())).unwrap();
    let swapped = one_file(&first.name, second.bytes.clone());
    assert!(
        matches!(document.verify(&swapped), Err(FormatError::VerifyMismatch { .. })),
        "{}: verify accepts another slot",
        first.label
    );
}

#[test]
fn real_saves_identify_their_game() {
    for (folder, game) in GAMES.iter().zip(neoffse_core::Game::ALL) {
        let Some(paths) = save_files(folder) else { continue };
        let (mut slots, mut opened, mut others) = (0, 0, 0);
        for path in paths {
            let name = path.file_name().unwrap().to_string_lossy().into_owned();
            let bytes = std::fs::read(&path).unwrap();
            let label = format!("{folder} {}", name.chars().take(8).collect::<String>());
            let Ok(decoded) = decode(&bytes) else { panic!("{label}: decode failed") };
            let Ok(mut document) = Document::parse(&decoded.json) else {
                panic!("{label}: parse failed")
            };
            let outcome = identify(&mut document);
            let opens = Ff1Format::new().open_document(&one_file(&name, bytes)).is_ok();
            if holds_user_data(&decoded.json) {
                slots += 1;
                assert!(matches!(outcome, Ok(found) if found == *game), "{label}: wrong game");
                assert_eq!(opens, *game == neoffse_core::Game::Ff1Pr, "{label}: the FF I format's decision is wrong");
                opened += usize::from(opens);
            } else {
                others += 1;
                assert!(matches!(outcome, Err(OpenError::NotASlot { .. })), "{label}: a non-slot file is identified");
                assert!(!opens, "{label}: a non-slot file opens");
            }
        }
        report(&format!(
            "{folder}: {slots} slot files identified as {game:?}, {opened} opened by the FF I format, {others} other files refused"
        ));
    }
}
