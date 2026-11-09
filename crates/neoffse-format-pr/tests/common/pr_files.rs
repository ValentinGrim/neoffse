//! Test helpers on the files of a Pixel Remaster game: the error that
//! refuses an open, the container shapes, the committed fixtures and two
//! small readers on a parsed slot. FF I to FF VI take the same calls.

use std::collections::HashSet;

use neoffse_core::FormatError;
use neoffse_format_pr::OpenError;
use neoffse_format_pr::container::Container;
use neoffse_format_pr::document::{Document, Scalar, Segment};

/// The source of a `Decode` error; any other result panics.
pub(crate) fn source_of<T>(result: Result<T, FormatError>) -> Box<dyn std::error::Error + Send + Sync> {
    match result {
        Err(FormatError::Decode { source }) => source,
        Err(other) => panic!("expected Decode, got {other:?}"),
        Ok(_) => panic!("expected an error"),
    }
}

/// The [`OpenError`] under the `Decode` error of `result`.
pub(crate) fn open_error<T>(result: Result<T, FormatError>) -> OpenError {
    *source_of(result).downcast::<OpenError>().expect("the source is an OpenError")
}

/// The three container shapes a slot file takes: the game's own, a BOM with
/// no trailer, and no BOM with a whitespace trailer
/// (`docs/formats/pixel-remaster.md#slot-files`).
pub(crate) fn containers() -> [Container; 3] {
    [
        Container::game(),
        Container { bom: true, trailer: vec![] },
        Container {
            bom: false,
            trailer: b" \r\n\t".to_vec(),
        },
    ]
}

/// True when no key of `keys` repeats.
pub(crate) fn unique(keys: &[String]) -> bool {
    keys.iter().collect::<HashSet<_>>().len() == keys.len()
}

/// The string at `path`; any other scalar panics.
pub(crate) fn text_at(document: &mut Document, path: &[Segment<'_>]) -> String {
    let Scalar::Text(text) = document.get(path).unwrap() else {
        panic!("no string at the path")
    };
    text
}

/// Fails when the committed fixture `file` differs byte for byte from `text`.
pub(crate) fn assert_fixture_matches(file: &str, text: &str) {
    let committed = neoffse_testkit::read_fixture(file).unwrap();
    assert!(committed == text.as_bytes(), "{file} differs from the generator: regenerate it");
}

/// Writes `text` to the fixture `file`, only when `NEOFFSE_WRITE_FIXTURES`
/// is set: the regeneration command of a fixture test sets it.
pub(crate) fn write_fixture(file: &str, text: &str) {
    if std::env::var_os("NEOFFSE_WRITE_FIXTURES").is_none() {
        return;
    }
    let path = neoffse_testkit::fixture_path(file);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, text).unwrap();
}
