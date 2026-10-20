//! A relative path in the `Environment` counts as absent: discovery searches
//! nothing there and reports nothing, although the path resolves against the
//! current folder of the process.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use std::fs;
use std::path::{Component, Path, PathBuf};

use common::discovery::{GAME_A, ID_1, ID_2, assert_nothing, documents_save, paths, prefix, site, steam_root};
use neoffse_io::discovery::{Environment, discover};

/// Builds an `Environment` that sets one field to the given path.
type Field = fn(PathBuf) -> Environment;

/// `target` spelled relative to the current folder: one `..` per component of
/// the current folder, then the components of `target` without its root.
/// `None` when the two have different prefixes (another drive on Windows).
fn relative(target: &Path) -> Option<PathBuf> {
    let current = fs::canonicalize(std::env::current_dir().unwrap()).unwrap();
    let (current_prefix, current_names) = split(&current);
    let (target_prefix, target_names) = split(target);
    if current_prefix != target_prefix {
        return None;
    }
    let mut spelled: PathBuf = current_names.iter().map(|_| "..").collect();
    spelled.extend(target_names);
    assert!(spelled.is_relative());
    assert!(spelled.exists(), "the relative spelling must resolve");
    Some(spelled)
}

/// The drive prefix of `path` and its plain names.
fn split(path: &Path) -> (Option<Component<'_>>, Vec<&std::ffi::OsStr>) {
    let prefix = path.components().find(|part| matches!(part, Component::Prefix(_)));
    let names = path
        .components()
        .filter_map(|part| match part {
            Component::Normal(name) => Some(name),
            _ => None,
        })
        .collect();
    (prefix, names)
}

#[test]
fn a_relative_home_is_not_searched() {
    let site = site("a_relative_home_is_not_searched");
    let root = steam_root(&site, ".local/share/Steam");
    let save = prefix(&root, &GAME_A, ID_1);
    let xdg = steam_root(&site, "xdg/Steam");
    prefix(&xdg, &GAME_A, ID_2);

    let control = Environment {
        home: Some(site.home.clone()),
        data_home: Some(site.home.join(".local/share")),
        ..Environment::default()
    };
    assert_eq!(paths(&discover(&control, &[GAME_A])), [save]);

    // Skipped on a platform where the temporary folder has no relative spelling.
    let (Some(home), Some(data_home)) = (relative(&site.home), relative(&site.home.join(".local/share"))) else {
        return;
    };
    let both = Environment {
        home: Some(home),
        data_home: Some(data_home),
        ..Environment::default()
    };
    assert_nothing(&discover(&both, &[GAME_A]));

    // The home holds no Steam root: only the relative data home could lead to one.
    let Some(xdg_home) = relative(&site.home.join("xdg")) else {
        return;
    };
    let data_only = Environment {
        home: Some(site.base.join("empty-home")),
        data_home: Some(xdg_home),
        ..Environment::default()
    };
    assert_nothing(&discover(&data_only, &[GAME_A]));
}

#[test]
fn a_relative_documents_is_not_searched() {
    let site = site("a_relative_documents_is_not_searched");
    let documents = site.base.join("docs");
    let profile = site.base.join("profile");
    let drive = site.base.join("drive");
    documents_save(&documents, &GAME_A, ID_1);
    documents_save(&profile.join("Documents"), &GAME_A, ID_1);
    documents_save(&drive.join("Documents"), &GAME_A, ID_1);

    let fields: [(&Path, Field); 3] = [
        (&documents, |path| Environment {
            documents: Some(path),
            ..Environment::default()
        }),
        (&profile, |path| Environment {
            user_profile: Some(path),
            ..Environment::default()
        }),
        (&drive, |path| Environment {
            one_drive: Some(path),
            ..Environment::default()
        }),
    ];
    for (folder, environment) in fields {
        let control = discover(&environment(folder.to_path_buf()), &[GAME_A]);
        assert_eq!(control.folders.len(), 1);

        // Skipped on a platform where the temporary folder has no relative spelling.
        let Some(spelled) = relative(folder) else {
            return;
        };
        assert_nothing(&discover(&environment(spelled), &[GAME_A]));
    }
}
