//! Slot-file detection: what it claims, what it refuses and how sure it is.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

use std::collections::HashSet;

use neoffse_core::{Confidence, Detection, FileProbe, SaveSlot};
use neoffse_format_pr::slots::{SLOT_FILES, detect, slot_for_id, slot_for_name};

/// 44 base64 characters, the shape of the head of a real file.
const HEAD: &[u8] = b"ZGVhZGJlZWZkZWFkYmVlZmRlYWRiZWVmZGVhZGJlZWY=\r\n";

fn probe<'a>(name: &'a str, head: &'a [u8]) -> FileProbe<'a> {
    FileProbe::new(name, head.len() as u64, head)
}

fn expected_slot(id: u32) -> SaveSlot {
    match id {
        21 => SaveSlot::Auto,
        22 => SaveSlot::Quick,
        n => SaveSlot::Numbered(n),
    }
}

/// The first cell of every data row under `### Other files in the folder`,
/// backticks stripped: the names the negatives read from the document.
fn other_file_names() -> Vec<String> {
    let path = neoffse_testkit::workspace_root().join("docs/formats/pixel-remaster.md");
    let document = std::fs::read_to_string(path).unwrap();
    document
        .lines()
        .skip_while(|line| line.trim_end() != "### Other files in the folder")
        .skip(1)
        .skip_while(|line| !line.starts_with('|'))
        .take_while(|line| line.starts_with('|'))
        .skip(2)
        .map(|line| line.trim().trim_matches('|').split('|').next().unwrap().trim().replace('`', ""))
        .collect()
}

#[test]
fn every_slot_name_is_detected_with_its_slot() {
    for file in SLOT_FILES {
        let detection = detect(&probe(file.name, HEAD)).unwrap();
        assert_eq!(detection.confidence, Confidence::Likely, "slot {}", file.id);
        assert_eq!(detection.slot, Some(expected_slot(file.id)), "slot {}", file.id);
    }
}

#[test]
fn a_bom_is_optional() {
    let mut with_bom = vec![0xEF, 0xBB, 0xBF];
    with_bom.extend_from_slice(HEAD);
    let name = SLOT_FILES[2].name;
    let plain = detect(&probe(name, HEAD)).unwrap();
    let bom = detect(&probe(name, &with_bom)).unwrap();
    assert_eq!(plain.confidence, Confidence::Likely);
    assert_eq!(bom, plain);
}

#[test]
fn a_head_without_line_end_is_still_likely() {
    let detection = detect(&probe(SLOT_FILES[0].name, b"QUJD")).unwrap();
    assert_eq!(detection.confidence, Confidence::Likely);
}

#[test]
fn a_slot_name_with_a_foreign_head_is_only_possible() {
    let name = SLOT_FILES[3].name;
    for (label, head) in [
        ("json", &b"{\"id\":1}"[..]),
        ("binary", &[0x00, 0x9F, 0x92, 0xFF][..]),
        ("space inside", b"QUJD REVG"),
        ("newline inside", b"QUJD\r\nREVG"),
        ("bom only", &[0xEF, 0xBB, 0xBF][..]),
        ("whitespace only", b" \r\n"),
        ("bom after a byte", b"A\xEF\xBB\xBFB"),
    ] {
        let detection = detect(&probe(name, head)).unwrap();
        assert_eq!(detection.confidence, Confidence::Possible, "{label}");
        assert_eq!(detection.slot, Some(SaveSlot::Numbered(2)), "{label}");
    }
}

#[test]
fn an_empty_file_with_a_slot_name_is_possible() {
    let detection = detect(&FileProbe::new(SLOT_FILES[2].name, 0, &[])).unwrap();
    assert_eq!(detection.confidence, Confidence::Possible);
    assert_eq!(detection.slot, Some(SaveSlot::Numbered(1)));
}

#[test]
fn detection_never_claims_certainty() {
    for file in SLOT_FILES {
        for head in [HEAD, b"", b"{}"] {
            let detection: Detection = detect(&probe(file.name, head)).unwrap();
            assert!(detection.confidence < Confidence::Certain);
        }
    }
}

#[test]
fn the_temporary_file_of_the_atomic_write_is_refused() {
    let slot = SLOT_FILES[2].name;
    let names = [".neoffse-AbC123.tmp".to_owned(), format!("{slot}.tmp"), format!(".neoffse-{slot}.tmp")];
    for name in names {
        assert_eq!(detect(&probe(&name, HEAD)), None, "{name}");
        assert_eq!(slot_for_name(&name), None, "{name}");
    }
}

#[test]
fn other_game_files_are_refused() {
    let mut names = other_file_names();
    assert_eq!(names.len(), 4, "the document lists four other files: {names:?}");
    assert!(names.iter().any(|name| name == "steam_autocloud.vdf"));
    names.retain(|name| !name.is_empty());
    for name in names {
        assert_eq!(detect(&probe(&name, HEAD)), None, "{name}");
    }
}

#[test]
fn near_misses_are_refused() {
    let slot = SLOT_FILES[2].name;
    for name in [
        slot.to_ascii_lowercase(),
        slot.to_ascii_uppercase(),
        format!("{slot} "),
        format!(" {slot}"),
        format!("./{slot}"),
        format!("dir/{slot}"),
        slot[..43].to_owned(),
        "slot1.sav".to_owned(),
        "slot22.sav".to_owned(),
        String::new(),
    ] {
        assert_eq!(detect(&probe(&name, HEAD)), None, "{name:?}");
        assert_eq!(slot_for_name(&name), None, "{name:?}");
    }
}

#[test]
fn slot_for_id_follows_the_table() {
    for file in SLOT_FILES {
        assert_eq!(slot_for_id(i64::from(file.id)), slot_for_name(file.name), "slot {}", file.id);
        assert_eq!(slot_for_id(i64::from(file.id)), Some(expected_slot(file.id)));
    }
    for id in [0, 23, -1, 100, i64::from(u32::MAX) + 1, i64::MAX, i64::MIN] {
        assert_eq!(slot_for_id(id), None, "id {id}");
    }
}

#[test]
fn names_and_ids_are_unique() {
    let ids: Vec<u32> = SLOT_FILES.iter().map(|file| file.id).collect();
    let expected: Vec<u32> = [21, 22].into_iter().chain(1..=20).collect();
    assert_eq!(ids, expected);
    let names: HashSet<_> = SLOT_FILES.iter().map(|file| file.name).collect();
    assert_eq!(names.len(), SLOT_FILES.len());
}

#[test]
fn the_probe_keeps_only_the_first_256_bytes() {
    let mut head = vec![b'A'; 256];
    head.extend_from_slice(b" not base64");
    let probe = FileProbe::new(SLOT_FILES[2].name, 1000, &head);
    assert_eq!(detect(&probe).unwrap().confidence, Confidence::Likely);
}
