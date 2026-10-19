//! The library paths of a Steam `libraryfolders.vdf`.
//!
//! [`library_paths`] reads the text of the file and touches no disk. The
//! top-level key must be `libraryfolders`. Each child whose key is a number
//! and whose value is a block with a non-empty `path` string is a library.
//! The function ignores every other child without an error: string children
//! (the layout of old Steam versions), non-numeric keys such as
//! `contentstatsid`, blocks without `path`, an empty `path`, a `path` that is
//! a block, and the other keys of a library (`label`, `totalsize`, `apps`).
//!
//! Libraries come back in the numeric order of their keys, not in the text
//! order of the parser's map. A key that appears twice, or two spellings of
//! one number, keep every block. A path is returned as written, unescaped:
//! the parser turns `\\` into `\`, which is how Steam writes a Windows path.
//! The function does not canonicalize a path or check that it exists.
//!
//! The error text of a malformed file quotes the line that failed to parse,
//! with its line and column. That line can hold a path of the user's home,
//! which is personal data: never log the error.

use std::error::Error;
use std::path::PathBuf;

use keyvalues_parser::Value;

const TOP_KEY: &str = "libraryfolders";

/// The library paths named by the text of a `libraryfolders.vdf`.
///
/// The paths come in the numeric order of their keys; see the
/// [module](self) for what counts as a library and what the function ignores.
///
/// # Errors
///
/// Returns the parser error when the text is not valid VDF, and a message
/// when the top-level key is not `libraryfolders` or holds a string instead
/// of a block. The parser error quotes the line that failed to parse, and that
/// line can hold a path of the user's home: do not log it.
pub fn library_paths(text: &str) -> Result<Vec<PathBuf>, Box<dyn Error + Send + Sync>> {
    let vdf = keyvalues_parser::parse(text)?.into_vdf();

    // top key: pixel-remaster.md#steam-roots-and-libraries
    if &*vdf.key != TOP_KEY {
        return Err(format!("expected top key {TOP_KEY:?}, found {:?}", &*vdf.key).into());
    }
    let Some(folders) = vdf.value.get_obj() else {
        return Err(format!("{TOP_KEY:?} holds a string, not a block").into());
    };

    let mut libraries: Vec<(u32, PathBuf)> = folders
        .iter()
        .filter_map(|(key, values)| key.parse::<u32>().ok().map(|number| (number, values)))
        .flat_map(|(number, values)| values.iter().filter_map(library_path).map(move |path| (number, path)))
        .collect();

    // numeric order: the map sorts keys as text, pixel-remaster.md#steam-roots-and-libraries
    libraries.sort_by_key(|(number, _)| *number);
    Ok(libraries.into_iter().map(|(_, path)| path).collect())
}

/// The `path` of a library block, or `None` when `value` is not a block or
/// its first `path` is not a non-empty string.
fn library_path(value: &Value<'_>) -> Option<PathBuf> {
    let path = value.get_obj()?.get("path")?.first()?.get_str()?;
    (!path.is_empty()).then(|| PathBuf::from(path))
}
