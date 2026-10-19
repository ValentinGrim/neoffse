//! Creating a backup: layout, manifest, collision suffix, validation and the
//! order of the writes.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use std::collections::BTreeSet;
use std::fs;

use common::backup::{BESTIARY, SLOT, Snoop, T0, at, game_dir, listing, make, request, tree, world};
use common::{args, names};
use neoffse_io::atomic::RealFileOps;
use neoffse_io::backup::{Manifest, Moment, Request, create, create_with};
use neoffse_io::dirs::DataDir;
use neoffse_io::error::IoError;

#[test]
fn backup_lands_in_the_documented_layout() {
    let w = world("backup_lands_in_the_documented_layout");
    let source_before = tree(&w.source);
    let files = [SLOT, BESTIARY];
    let now = Moment::new(T0, 7_200).unwrap();
    let backup = create_with(&RealFileOps, &w.data, &request(&w, "ff1-pr", &files), now).unwrap();

    let expected = w.root.join("data").join("backups").join("ff1-pr").join("20261018-221403");
    assert_eq!(backup.dir, expected);
    let found = tree(&backup.dir);
    let keys: BTreeSet<&str> = found.keys().map(String::as_str).collect();
    assert_eq!(keys, BTreeSet::from(["backup.json", SLOT, BESTIARY]));
    assert_eq!(found[SLOT], b"slot-one");
    assert_eq!(found[BESTIARY], b"bestiary-one");
    assert_eq!(tree(&w.source), source_before);
}

#[test]
fn manifest_holds_the_documented_fields() {
    let w = world("manifest_holds_the_documented_fields");
    let files = [SLOT, BESTIARY];
    let now = Moment::new(T0, 7_200).unwrap();
    let backup = create_with(&RealFileOps, &w.data, &request(&w, "ff1-pr", &files), now).unwrap();

    let bytes = fs::read(backup.dir.join("backup.json")).unwrap();
    let text = String::from_utf8(bytes.clone()).unwrap();
    assert!(text.starts_with("{\n  \"version\": 1,\n  \"game\": \"ff1-pr\",\n"), "{text}");
    assert!(text.ends_with("}\n"), "{text}");

    let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let object = value.as_object().unwrap();
    let keys: BTreeSet<&str> = object.keys().map(String::as_str).collect();
    let expected = ["version", "game", "sourceDir", "created", "files", "saveType", "reason"];
    assert_eq!(keys, BTreeSet::from(expected));
    assert_eq!(object["version"], 1);
    assert_eq!(object["sourceDir"], w.source.to_string_lossy().as_ref());
    assert_eq!(object["created"], "2026-10-18T22:14:03+02:00");
    assert_eq!(object["files"], serde_json::json!([SLOT, BESTIARY]));
    assert_eq!(object["saveType"], "slot");
    assert_eq!(object["reason"], "before-save");
    assert_eq!(backup.manifest, serde_json::from_slice::<Manifest>(&bytes).unwrap());
}

#[test]
fn moment_formats_known_instants() {
    let rows: [(i64, i32, &str, &str); 11] = [
        (0, 0, "19700101-000000", "1970-01-01T00:00:00Z"),
        (951_782_400, 0, "20000229-000000", "2000-02-29T00:00:00Z"),
        (1_709_164_800, 0, "20240229-000000", "2024-02-29T00:00:00Z"),
        (4_107_542_399, 0, "21000228-235959", "2100-02-28T23:59:59Z"),
        (4_107_542_400, 0, "21000301-000000", "2100-03-01T00:00:00Z"),
        (253_402_300_799, 0, "99991231-235959", "9999-12-31T23:59:59Z"),
        (T0, 0, "20261018-201403", "2026-10-18T20:14:03Z"),
        (T0, 7_200, "20261018-221403", "2026-10-18T22:14:03+02:00"),
        (T0, -3_600, "20261018-191403", "2026-10-18T19:14:03-01:00"),
        (T0, 19_800, "20261019-014403", "2026-10-19T01:44:03+05:30"),
        (18_000, -18_000, "19700101-000000", "1970-01-01T00:00:00-05:00"),
    ];
    for (n, (unix, offset, folder, created)) in rows.into_iter().enumerate() {
        let w = world(&format!("moment_formats_known_instants-{n}"));
        let files = [SLOT];
        let now = Moment::new(unix, offset).unwrap();
        let backup = create_with(&RealFileOps, &w.data, &request(&w, "ff1-pr", &files), now).unwrap();
        assert_eq!(backup.dir.file_name().unwrap().to_str().unwrap(), folder, "row {n}");
        assert_eq!(backup.manifest.created, created, "row {n}");
        let on_disk: Manifest = serde_json::from_slice(&fs::read(backup.dir.join("backup.json")).unwrap()).unwrap();
        assert_eq!(on_disk.created, created, "row {n}");
    }
}

#[test]
fn moment_new_rejects_instants_the_name_cannot_hold() {
    for (unix, offset) in [(-1, 0), (0, -60), (0, 90), (0, 86_400), (0, -86_400), (253_402_300_800, 0)] {
        assert!(Moment::new(unix, offset).is_none(), "({unix}, {offset})");
    }
    for (unix, offset) in [(0, 0), (0, 86_340), (86_340, -86_340), (253_402_300_799, 0)] {
        assert!(Moment::new(unix, offset).is_some(), "({unix}, {offset})");
    }
}

#[test]
fn same_second_backups_get_a_suffix() {
    let w = world("same_second_backups_get_a_suffix");
    let mut dirs = Vec::new();
    let mut firsts = None;
    for n in 0..3 {
        fs::write(w.source.join(SLOT), format!("slot-{n}")).unwrap();
        dirs.push(make(&w, "ff1-pr", T0).dir);
        if n == 0 {
            firsts = Some(tree(&dirs[0]));
        }
    }
    let names: Vec<&str> = dirs.iter().map(|dir| dir.file_name().unwrap().to_str().unwrap()).collect();
    assert_eq!(names, ["20261018-201403", "20261018-201403-01", "20261018-201403-02"]);
    for (n, dir) in dirs.iter().enumerate() {
        assert_eq!(tree(dir)[SLOT], format!("slot-{n}").into_bytes());
    }
    assert_eq!(tree(&dirs[0]), firsts.unwrap());
}

#[test]
fn empty_and_large_files_copy_byte_for_byte() {
    let w = world("empty_and_large_files_copy_byte_for_byte");
    let large: Vec<u8> = (0..=255u8).cycle().take(256 * 400).collect();
    fs::write(w.source.join("empty"), b"").unwrap();
    fs::write(w.source.join("large"), &large).unwrap();
    let files = ["empty", "large"];
    let backup = create_with(&RealFileOps, &w.data, &request(&w, "ff1-pr", &files), at(T0)).unwrap();
    assert_eq!(backup.manifest.files, ["empty", "large"]);
    let found = tree(&backup.dir);
    assert_eq!(found["empty"], Vec::<u8>::new());
    assert_eq!(found["large"], large);
}

#[test]
fn invalid_requests_fail_before_any_disk_access() {
    type Case = (Vec<&'static str>, &'static str, &'static str, &'static str);
    let plain = "not a plain file name";
    let mut cases: Vec<Case> = vec![(vec![], "slot", "before-save", "at least one file")];
    for name in ["", ".", "..", "../x", "a/b", "a\\b", "c:x", "backup.json", "BACKUP.JSON"] {
        cases.push((vec![name], "slot", "before-save", plain));
    }
    cases.push((vec![SLOT, SLOT], "slot", "before-save", "appears twice"));
    cases.push((vec!["Ab", "aB"], "slot", "before-save", "appears twice"));
    cases.push((vec![SLOT], "slot", "", "reason is empty"));
    cases.push((vec![SLOT], "", "before-save", "saveType is empty"));

    for (n, (files, save_type, reason, phrase)) in cases.into_iter().enumerate() {
        let w = world(&format!("invalid_requests_fail_before_any_disk_access-{n}"));
        let request = Request {
            game: "ff1-pr",
            source_dir: &w.source,
            files: &files,
            save_type,
            reason,
        };
        let error = create_with(&RealFileOps, &w.data, &request, at(T0)).unwrap_err();
        let IoError::ManifestInvalid { path, source } = &error else {
            panic!("case {n}: {error:?}");
        };
        assert!(path.ends_with("backup.json"), "case {n}");
        assert!(source.to_string().contains(phrase), "case {n}: {source}");
        assert!(!w.data.root().exists(), "case {n}");
    }
}

#[test]
fn invalid_game_id_fails_before_any_disk_access() {
    for (n, game) in ["", "FF1", "../x", "a/b"].into_iter().enumerate() {
        let w = world(&format!("invalid_game_id_fails_before_any_disk_access-{n}"));
        let files = [SLOT];
        let error = create_with(&RealFileOps, &w.data, &request(&w, game, &files), at(T0)).unwrap_err();
        assert!(matches!(error, IoError::InvalidGameId { .. }), "{game:?}: {error:?}");
        assert!(!w.data.root().exists(), "{game:?}");
    }
}

#[test]
fn manifest_is_written_last_through_the_atomic_pipeline() {
    let w = world("manifest_is_written_last_through_the_atomic_pipeline");
    let (snoop, log) = Snoop::new(&[], false);
    let files = [SLOT, BESTIARY];
    let backup = create_with(&snoop, &w.data, &request(&w, "ff1-pr", &files), at(T0)).unwrap();

    let mut copies = vec![SLOT.to_owned(), BESTIARY.to_owned()];
    copies.sort();
    assert_eq!(listing(&log), copies);
    let order = [
        "listing",
        "create_temp",
        "write_all",
        "copy_permissions",
        "sync_file",
        "rename",
        "sync_dir",
        "sync_dir",
    ];
    assert_eq!(names(&log), order);

    let dir = backup.dir.to_string_lossy().into_owned();
    assert_eq!(args(&log, "create_temp")[0], dir);
    assert_eq!(args(&log, "rename")[1], backup.dir.join("backup.json").to_string_lossy());
    let syncs: Vec<String> = log
        .borrow()
        .iter()
        .filter_map(|entry| entry.strip_prefix("sync_dir|").map(str::to_owned))
        .collect();
    assert_eq!(syncs, [dir, game_dir(&w, "ff1-pr").to_string_lossy().into_owned()]);
}

#[cfg(unix)]
#[test]
fn non_utf8_source_dir_still_backs_up() {
    use std::ffi::OsStr;
    use std::os::unix::ffi::OsStrExt as _;

    let w = world("non_utf8_source_dir_still_backs_up");
    let source = w.root.join(OsStr::from_bytes(b"src-\xff"));
    fs::create_dir_all(&source).unwrap();
    fs::write(source.join(SLOT), b"slot-one").unwrap();
    let files = [SLOT];
    let request = Request {
        game: "ff1-pr",
        source_dir: &source,
        files: &files,
        save_type: "slot",
        reason: "before-save",
    };
    let backup = create_with(&RealFileOps, &w.data, &request, at(T0)).unwrap();
    assert!(backup.manifest.source_dir.contains('\u{FFFD}'));
    assert!(backup.dir.join("backup.json").is_file());
}

#[test]
fn create_uses_the_system_clock() {
    let w = world("create_uses_the_system_clock");
    let data = DataDir::at(w.root.join("clock-data"));
    let files = [SLOT];
    let backup = create(&data, &request(&w, "ff1-pr", &files)).unwrap();

    let name = backup.dir.file_name().unwrap().to_str().unwrap();
    let digits = |part: &str| part.bytes().all(|b| b.is_ascii_digit());
    assert_eq!(name.len(), 15, "{name}");
    assert_eq!(name.as_bytes()[8], b'-', "{name}");
    assert!(digits(&name[..8]) && digits(&name[9..]), "{name}");
    assert!(name > "20260101-000000", "{name}");
    assert!(backup.manifest.created.ends_with('Z'), "{}", backup.manifest.created);
    assert!(backup.manifest.created.starts_with("20"), "{}", backup.manifest.created);
}
