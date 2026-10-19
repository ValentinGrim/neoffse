//! `matching`: both keys, the separators, the case rule, whole-name equality,
//! threads and the shape of the result. No real process is involved.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

use neoffse_io::process::{Key, ProcessInfo, matching};

/// The six executable names of docs/formats/pixel-remaster.md#games.
const SIX: [&str; 6] = [
    "FINAL FANTASY.exe",
    "FINAL FANTASY II.exe",
    "FINAL FANTASY III.exe",
    "FINAL FANTASY IV.exe",
    "FINAL FANTASY V.exe",
    "FINAL FANTASY VI.exe",
];

/// No executable matched.
const NONE: [&str; 0] = [];

const FF1: &str = "FINAL FANTASY.exe";
const FF2: &str = "FINAL FANTASY II.exe";
const FF2_PATH: &str = r"S:\steamapps\common\FINAL FANTASY II PR\FINAL FANTASY II.exe";

fn process(name: &str, argv0: Option<&str>, is_thread: bool) -> ProcessInfo {
    ProcessInfo {
        name: name.to_owned(),
        argv0: argv0.map(str::to_owned),
        is_thread,
    }
}

/// A process the way Linux reports a game: the 15-byte `comm` and the Windows path.
fn game(name: &str, argv0: &str) -> ProcessInfo {
    process(name, Some(argv0), false)
}

#[test]
fn command_line_key_matches_the_windows_path_of_argv0() {
    let table = [game("FINAL FANTASY I", FF2_PATH)];
    assert_eq!(matching(&table, Key::CommandLine, &SIX), [FF2]);
}

#[test]
fn separators_slash_backslash_and_mixed_all_split() {
    for (argv0, expected) in [
        ("/home/u/FINAL FANTASY V.exe", "FINAL FANTASY V.exe"),
        (r"Z:/games\FINAL FANTASY VI.exe", "FINAL FANTASY VI.exe"),
        ("FINAL FANTASY.exe", FF1),
    ] {
        let table = [game("x", argv0)];
        assert_eq!(matching(&table, Key::CommandLine, &SIX), [expected]);
    }
}

#[test]
fn command_line_key_never_reads_the_name() {
    let truncated = [process("FINAL FANTASY I", None, false)];
    assert_eq!(matching(&truncated, Key::CommandLine, &SIX), NONE);
    let exact = [process("a.exe", None, false)];
    assert_eq!(matching(&exact, Key::CommandLine, &["a.exe"]), NONE);
}

#[test]
fn name_key_matches_the_name_and_never_reads_argv0() {
    let table = [process("FINAL FANTASY III.exe", None, false)];
    assert_eq!(matching(&table, Key::Name, &SIX), ["FINAL FANTASY III.exe"]);
    let loader = [game("wine64-preloader", FF2_PATH)];
    assert_eq!(matching(&loader, Key::Name, &SIX), NONE);
}

#[test]
fn case_is_ignored_and_the_callers_spelling_is_returned() {
    let by_name = [process("final fantasy ii.EXE", None, false)];
    assert_eq!(matching(&by_name, Key::Name, &SIX), [FF2]);
    let by_path = [game("x", r"c:\x\FINAL fantasy VI.Exe")];
    assert_eq!(matching(&by_path, Key::CommandLine, &SIX), ["FINAL FANTASY VI.exe"]);
}

#[test]
fn case_folding_is_ascii_only() {
    let kelvin = [process("\u{212A}ERNEL.exe", None, false)];
    assert_eq!(matching(&kelvin, Key::Name, &["kernel.exe"]), NONE);
    let sign_in_path = [game("x", "/a/\u{212A}ERNEL.exe")];
    assert_eq!(matching(&sign_in_path, Key::CommandLine, &["kernel.exe"]), NONE);
}

#[test]
fn the_whole_name_must_be_equal() {
    for argv0 in [
        r"C:\x\NOT FINAL FANTASY.exe",
        r"C:\x\FINAL FANTASY.exe.bak",
        r"C:\x\FINAL FANTASY.exe\",
        r"C:\x\FINAL FANTASY.exe ",
        "FINAL  FANTASY.exe",
    ] {
        let table = [game("x", argv0)];
        assert_eq!(matching(&table, Key::CommandLine, &[FF1]), NONE);
    }
    let prefixed = [process("XFINAL FANTASY.exe", None, false)];
    assert_eq!(matching(&prefixed, Key::Name, &[FF1]), NONE);
}

#[test]
fn one_game_never_matches_another() {
    let ff2 = [game("x", FF2_PATH)];
    assert_eq!(matching(&ff2, Key::CommandLine, &SIX), [FF2]);
    assert_eq!(matching(&ff2, Key::CommandLine, &[FF2]), [FF2]);
    assert_eq!(matching(&ff2, Key::CommandLine, &[FF1]), NONE);
    let ff1 = [game("x", r"S:\steamapps\common\FINAL FANTASY PR\FINAL FANTASY.exe")];
    assert_eq!(matching(&ff1, Key::CommandLine, &[FF2]), NONE);
    assert_eq!(matching(&ff1, Key::CommandLine, &SIX), [FF1]);
}

#[test]
fn threads_never_match() {
    let by_name = |is_thread| [process(FF2, None, is_thread)];
    assert_eq!(matching(&by_name(true), Key::Name, &SIX), NONE);
    assert_eq!(matching(&by_name(false), Key::Name, &SIX), [FF2]);
    let by_path = |is_thread| [process("x", Some(FF2_PATH), is_thread)];
    assert_eq!(matching(&by_path(true), Key::CommandLine, &SIX), NONE);
    assert_eq!(matching(&by_path(false), Key::CommandLine, &SIX), [FF2]);
}

#[test]
fn a_thread_does_not_hide_the_real_process() {
    let table = [process("x", Some(FF2_PATH), true), process("x", Some(FF2_PATH), false)];
    assert_eq!(matching(&table, Key::CommandLine, &SIX), [FF2]);
}

#[test]
fn the_result_follows_the_executables_not_the_processes() {
    let ff6 = "FINAL FANTASY VI.exe";
    let ff1 = "FINAL FANTASY.exe";
    let table = [game("x", FF2_PATH), game("x", r"C:\g\FINAL FANTASY VI.exe")];
    assert_eq!(matching(&table, Key::CommandLine, &[ff6, ff1, FF2, FF2]), [ff6, FF2, FF2]);
    let twice = [game("x", FF2_PATH), game("y", FF2_PATH)];
    assert_eq!(matching(&twice, Key::CommandLine, &[FF2]), [FF2]);
}

#[test]
fn an_empty_executable_name_matches_nothing() {
    let by_name = [process("", None, false)];
    assert_eq!(matching(&by_name, Key::Name, &[""]), NONE);
    let by_path = [game("", ""), game("", r"C:\x\")];
    assert_eq!(matching(&by_path, Key::CommandLine, &[""]), NONE);
    assert_eq!(matching(&by_path, Key::Name, &[""]), NONE);
}

#[test]
fn noise_matches_nothing() {
    let table: Vec<ProcessInfo> = ["steam", "reaper", "pv-adverb", "wine64-preloader", "/usr/bin/bwrap", "explorer.exe"]
        .into_iter()
        .map(|text| game(text, text))
        .collect();
    assert_eq!(matching(&table, Key::CommandLine, &SIX), NONE);
    assert_eq!(matching(&table, Key::Name, &SIX), NONE);
    assert_eq!(matching(&[], Key::CommandLine, &SIX), NONE);
    assert_eq!(matching(&table, Key::Name, &[]), NONE);
    assert_eq!(matching(&[], Key::Name, &[]), NONE);
}

#[test]
fn the_platform_key_follows_the_system() {
    let expected = if cfg!(windows) { Key::Name } else { Key::CommandLine };
    assert_eq!(Key::platform(), expected);
}
