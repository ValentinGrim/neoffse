//! `library_paths`: the shape of the measured file, the numeric order, the
//! ignored children and the errors.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

use std::path::PathBuf;

use neoffse_io::discovery::library_list::library_paths;

/// One numbered block in the shape of the measured file.
fn block(key: &str, path: &str) -> String {
    format!(
        "\t\"{key}\"\n\t{{\n\t\t\"path\"\t\t\"{path}\"\n\t\t\"label\"\t\t\"\"\n\
         \t\t\"contentid\"\t\t\"1234567890\"\n\t\t\"totalsize\"\t\t\"500000000000\"\n\
         \t\t\"update_clean_bytes_tally\"\t\t\"42\"\n\
         \t\t\"time_last_update_verified\"\t\t\"1700000000\"\n\
         \t\t\"apps\"\n\t\t{{\n\t\t\t\"1173770\"\t\t\"9000\"\n\t\t}}\n\t}}\n"
    )
}

fn list(children: &str) -> String {
    format!("\"libraryfolders\"\n{{\n{children}}}\n")
}

fn paths(text: &str) -> Vec<PathBuf> {
    library_paths(text).unwrap()
}

fn paths_of(list: &[&str]) -> Vec<PathBuf> {
    list.iter().map(PathBuf::from).collect()
}

#[test]
fn the_real_shape_yields_every_library_path() {
    let text = list(&format!(
        "{}{}",
        block("0", "/home/u/.local/share/Steam"),
        block("1", "/mnt/jeux d\u{e9}/SteamLibrary"),
    ));
    assert_eq!(paths(&text), paths_of(&["/home/u/.local/share/Steam", "/mnt/jeux d\u{e9}/SteamLibrary"]));
}

#[test]
fn doubled_backslashes_are_unescaped() {
    let text = list("\t\"0\"\n\t{\n\t\t\"path\"\t\t\"D:\\\\SteamLibrary\"\n\t}\n");
    let found = paths(&text);
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].to_str().unwrap(), "D:\\SteamLibrary");
}

#[test]
fn libraries_follow_the_numeric_order_of_their_keys() {
    let text = list(&format!(
        "{}{}{}{}",
        block("0", "/lib/zero"),
        block("10", "/lib/ten"),
        block("2", "/lib/two"),
        block("1", "/lib/one"),
    ));
    assert_eq!(paths(&text), paths_of(&["/lib/zero", "/lib/one", "/lib/two", "/lib/ten"]));
}

#[test]
fn children_that_are_not_libraries_are_ignored() {
    let text = list(&format!(
        "\t\"contentstatsid\"\t\t\"123\"\n{}\t\"3\"\t\t\"/old/style\"\n\
         \t\"4\"\n\t{{\n\t\t\"label\"\t\t\"no path\"\n\t}}\n\
         \t\"5\"\n\t{{\n\t\t\"path\"\t\t\"\"\n\t}}\n\
         \t\"6\"\n\t{{\n\t\t\"path\"\n\t\t{{\n\t\t\t\"inner\"\t\t\"/in/block\"\n\t\t}}\n\t}}\n{}",
        block("foo", "/not/numeric"),
        block("7", "/the/valid/one"),
    ));
    assert_eq!(paths(&text), paths_of(&["/the/valid/one"]));
}

#[test]
fn an_empty_list_is_ok_and_empty() {
    assert_eq!(paths("\"libraryfolders\"\n{\n}\n"), Vec::<PathBuf>::new());
    assert_eq!(paths("\"libraryfolders\" { }"), Vec::<PathBuf>::new());
}

#[test]
fn duplicate_keys_keep_every_library() {
    let text = list(&format!("{}{}", block("1", "/first"), block("1", "/second")));
    assert_eq!(paths(&text), paths_of(&["/first", "/second"]));
}

#[test]
fn a_block_with_two_paths_takes_the_first() {
    let text = list("\t\"0\"\n\t{\n\t\t\"path\"\t\t\"/first\"\n\t\t\"path\"\t\t\"/second\"\n\t}\n");
    assert_eq!(paths(&text), paths_of(&["/first"]));
}

#[test]
fn a_wrong_top_key_is_an_error() {
    let wrong_case = library_paths("\"LibraryFolders\" { }").unwrap_err().to_string();
    assert!(wrong_case.contains("LibraryFolders"), "{wrong_case}");
    let other = library_paths("\"users\" { }").unwrap_err().to_string();
    assert!(other.contains("users"), "{other}");
    let string = library_paths("\"libraryfolders\" \"text\"").unwrap_err().to_string();
    assert!(string.contains("libraryfolders"), "{string}");
}

#[test]
fn malformed_text_is_an_error() {
    for text in [
        "",
        "  \n\t ",
        "\"libraryfolders\"\n{\n\t\"0\"\n\t{\n",
        "\"libraryfolders\n{\n}",
        "not vdf at all",
    ] {
        assert!(library_paths(text).is_err(), "{text:?}");
    }
}

#[test]
fn the_error_is_send_sync_and_displays() {
    fn assert_send_sync<T: Send + Sync>(_: &T) {}
    let error = library_paths("not vdf at all").unwrap_err();
    assert_send_sync(&error);
    assert_ne!(error.to_string(), "");
}
