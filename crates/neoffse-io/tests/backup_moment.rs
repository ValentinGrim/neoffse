//! `Moment::parse` and the accessors: the inverse of the text `create` writes.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use std::fs::File;
use std::io;
use std::path::{Path, PathBuf};

use common::backup::{SLOT, T0, request, world};
use neoffse_io::atomic::{FileOps, RealFileOps};
use neoffse_io::backup::{Moment, create, create_with};

/// 9999-12-31T23:59:59Z.
const MAX_SECONDS: i64 = 253_402_300_799;

/// `RealFileOps` without the disk flushes, so thousands of backups stay fast.
#[derive(Debug)]
struct NoSync;

impl FileOps for NoSync {
    fn create_temp(&self, dir: &Path) -> io::Result<(File, PathBuf)> {
        RealFileOps.create_temp(dir)
    }
    fn write_all(&self, file: &mut File, bytes: &[u8]) -> io::Result<()> {
        RealFileOps.write_all(file, bytes)
    }
    fn copy_permissions(&self, from: &Path, file: &File) -> io::Result<()> {
        RealFileOps.copy_permissions(from, file)
    }
    fn sync_file(&self, _file: &File) -> io::Result<()> {
        Ok(())
    }
    fn rename(&self, from: &Path, to: &Path) -> io::Result<()> {
        RealFileOps.rename(from, to)
    }
    fn remove_file(&self, path: &Path) -> io::Result<()> {
        RealFileOps.remove_file(path)
    }
    fn sync_dir(&self, _dir: &Path) -> io::Result<()> {
        Ok(())
    }
}

fn parsed(text: &str) -> (i64, i32) {
    let moment = Moment::parse(text).unwrap_or_else(|| panic!("{text} should parse"));
    (moment.unix_seconds(), moment.offset_seconds())
}

#[test]
fn accessors_return_what_new_received() {
    let moment = Moment::new(T0, 7_200).unwrap();
    assert_eq!(moment.unix_seconds(), T0);
    assert_eq!(moment.offset_seconds(), 7_200);
    assert_eq!(Moment::now().offset_seconds(), 0);
}

#[test]
fn parse_reads_the_created_text_of_a_real_backup() {
    let w = world("moment_parse_real");
    let files = [SLOT];
    let now = Moment::new(T0, 7_200).unwrap();
    let backup = create_with(&RealFileOps, &w.data, &request(&w, "ff1-pr", &files), now).unwrap();
    assert_eq!(Moment::parse(&backup.manifest.created), Some(now));

    // The folder name is the UTC stamp of the same instant.
    let stamp = backup.dir.file_name().unwrap().to_str().unwrap().to_owned();
    let utc = format!(
        "{}-{}-{}T{}:{}:{}Z",
        &stamp[0..4],
        &stamp[4..6],
        &stamp[6..8],
        &stamp[9..11],
        &stamp[11..13],
        &stamp[13..15]
    );
    assert_eq!(Moment::parse(&utc).unwrap().unix_seconds(), now.unix_seconds());

    let live = create(&w.data, &request(&w, "ff1-pr", &files)).unwrap();
    let moment = Moment::parse(&live.manifest.created).unwrap();
    assert_eq!(moment.offset_seconds(), 0);
}

#[test]
fn parse_gives_the_expected_instants() {
    let rows: [(&str, i64, i32); 12] = [
        ("1970-01-01T00:00:00Z", 0, 0),
        ("2000-02-29T00:00:00Z", 951_782_400, 0),
        ("2024-02-29T00:00:00Z", 1_709_164_800, 0),
        ("2100-02-28T23:59:59Z", 4_107_542_399, 0),
        ("2100-03-01T00:00:00Z", 4_107_542_400, 0),
        ("9999-12-31T23:59:59Z", MAX_SECONDS, 0),
        ("2026-10-18T20:14:03Z", T0, 0),
        ("2026-10-18T22:14:03+02:00", T0, 7_200),
        ("2026-10-18T19:14:03-01:00", T0, -3_600),
        ("2026-10-19T01:44:03+05:30", T0, 19_800),
        ("1970-01-01T00:00:00-05:00", 18_000, -18_000),
        ("1970-01-02T23:59:00+23:59", 86_400, 86_340),
    ];
    for (text, unix, offset) in rows {
        assert_eq!(parsed(text), (unix, offset), "{text}");
    }
}

#[test]
fn parse_inverts_the_text_create_writes() {
    let w = world("moment_parse_loop");
    let files = [SLOT];
    let offsets = [0, 60, -60, 1_800, 3_600, -3_600, 19_800, -18_000, 34_200, 45_900, 86_340, -86_340];
    let mut cases = vec![(0, 0), (MAX_SECONDS, 0), (0, 86_340), (86_340, -86_340), (MAX_SECONDS - 86_340, 86_340)];
    let mut state: u64 = 0x9e37_79b9_7f4a_7c15;
    for n in 0..3_000_usize {
        state = state.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
        let unix = i64::try_from((state >> 16) % 253_402_300_800).unwrap();
        cases.push((unix, offsets[n % offsets.len()]));
    }
    let mut checked = 0;
    for (unix, offset) in cases {
        let Some(now) = Moment::new(unix, offset) else {
            continue;
        };
        let backup = create_with(&NoSync, &w.data, &request(&w, "ff1-pr", &files), now).unwrap();
        assert_eq!(Moment::parse(&backup.manifest.created), Some(now), "({unix}, {offset})");
        checked += 1;
    }
    assert!(checked > 2_900, "{checked}");
}

#[test]
fn parse_refuses_dates_and_times_that_do_not_exist() {
    for text in [
        "2026-02-30T00:00:00Z",
        "2026-02-29T00:00:00Z",
        "2100-02-29T00:00:00Z",
        "2026-04-31T00:00:00Z",
        "2026-13-01T00:00:00Z",
        "2026-00-10T00:00:00Z",
        "2026-10-00T00:00:00Z",
        "2026-10-32T00:00:00Z",
        "2026-10-18T24:00:00Z",
        "2026-10-18T23:60:00Z",
        "2026-10-18T23:59:60Z",
    ] {
        assert!(Moment::parse(text).is_none(), "{text}");
    }
    assert!(Moment::parse("2000-02-29T00:00:00Z").is_some());
}

#[test]
fn parse_refuses_a_shape_create_never_writes() {
    for text in [
        "",
        "2026-10-18T20:14:03",
        "2026-10-18T20:14:03.5Z",
        "2026-10-18T20:14:03.000+02:00",
        "2026-10-18T20:14:03z",
        "2026-10-18t20:14:03Z",
        "2026-10-18 20:14:03Z",
        "2026-10-18T20:14:03ZZ",
        "2026-10-18T20:14:03Z ",
        " 2026-10-18T20:14:03Z",
        "2026-10-18T20:14Z",
        "2026-1-18T20:14:03Z",
        "20261018T201403Z",
        "2026-10-18T20:14:03+0200",
        "2026-10-18T20:14:03+02",
        "2026-10-18T20:14:03+02:00:30",
        "2026-10-18T20:14:03+2:00",
        "2026-10-18T20:14:03+02:0x",
        "2026-10-18T20:14:03*02:00",
        "2026-10-18T20:14:03\u{2212}02:00",
        "+026-10-18T20:14:03Z",
        "２０２６-10-18T20:14:03Z",
    ] {
        assert!(Moment::parse(text).is_none(), "{text:?}");
    }
}

#[test]
fn parse_refuses_the_offsets_and_instants_new_refuses() {
    for text in [
        "2026-10-18T20:14:03+24:00",
        "2026-10-18T20:14:03-24:00",
        "2026-10-18T20:14:03+23:60",
        "2026-10-18T20:14:03+99:99",
        "1969-12-31T23:59:59Z",
        "1970-01-01T00:00:00+01:00",
        "9999-12-31T23:59:59-01:00",
        "0000-01-01T00:00:00Z",
    ] {
        assert!(Moment::parse(text).is_none(), "{text}");
    }
}

#[test]
fn parse_refuses_a_zero_offset_that_create_writes_as_z() {
    assert!(Moment::parse("2026-10-18T20:14:03+00:00").is_none());
    assert!(Moment::parse("2026-10-18T20:14:03-00:00").is_none());
    assert!(Moment::parse("2026-10-18T20:14:03Z").is_some());
}
