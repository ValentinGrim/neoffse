//! Named files, the probe a format detects from, and the claim it makes.
//!
//! A format sees names and bytes and never a path, so it cannot reach the
//! disk and cannot choose where a file lands (`docs/architecture.md#save-safety`).

use std::fmt;

use crate::error::FormatError;
use crate::view::SaveSlot;

/// How many bytes of a file's start [`FileProbe::head`] holds at most.
///
/// A contract between the registry and the formats, not a format rule. The
/// number covers a byte order mark and the first base64 lines of a file; a
/// format that needs more decodes the whole file in `SaveFormat::open`.
pub const PROBE_HEAD_LEN: usize = 256;

/// What `SaveFormat::detect` sees of a file: its name, its size and its first
/// bytes.
///
/// The registry reads a name and at most [`PROBE_HEAD_LEN`] bytes. The name
/// is a file name without a directory.
///
/// `Debug` prints save content: never log it.
///
/// [`SaveFormat::detect`]: crate::format::SaveFormat::detect
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct FileProbe<'a> {
    /// The file name, without a directory.
    pub name: &'a str,
    /// The size of the whole file in bytes.
    pub len: u64,
    /// The first bytes of the file. [`FileProbe::new`] keeps at most
    /// [`PROBE_HEAD_LEN`] bytes; a format reads no more than that.
    pub head: &'a [u8],
}

impl<'a> FileProbe<'a> {
    /// Keeps the first [`PROBE_HEAD_LEN`] bytes of `head`; `len` is the size
    /// of the whole file and is not compared with `head.len()`.
    #[must_use]
    pub fn new(name: &'a str, len: u64, head: &'a [u8]) -> Self {
        Self {
            name,
            len,
            head: head.get(..PROBE_HEAD_LEN).unwrap_or(head),
        }
    }
}

/// How sure a format is that a file is its own.
///
/// `Possible` means the name fits, `Likely` that the name and a cheap content
/// check fit, `Certain` that the first bytes prove it. A format may use fewer
/// levels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[non_exhaustive]
pub enum Confidence {
    /// The name fits.
    Possible,
    /// The name and a cheap content check fit.
    Likely,
    /// The first bytes prove it.
    Certain,
}

/// A format's claim on a file.
///
/// The registry keeps the claim with the highest [`confidence`](Detection::confidence).
/// The six Pixel Remaster formats earn the same claim on the same file name
/// (`docs/formats/pixel-remaster.md#slot-files`: the names repeat in all six
/// games), and the facade (M4) breaks that tie with the game of the save
/// folder.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct Detection {
    /// How sure the format is.
    pub confidence: Confidence,
    /// What the name alone tells about the slot, for sorting a folder
    /// listing without opening the files.
    pub slot: Option<SaveSlot>,
}

impl Detection {
    /// A claim of `confidence` with no slot.
    #[must_use]
    pub fn new(confidence: Confidence) -> Self {
        Self { confidence, slot: None }
    }
}

/// Named files, never paths: the input of `SaveFormat::open` and the output
/// of `SaveDocument::encode`.
///
/// A `SaveFiles` holds names and bytes and never a path, so a format cannot
/// reach the disk and a `Debug` print leaks no save content: `Debug` shows
/// the names and the byte counts only. Files keep their insertion order.
///
/// ```
/// use neoffse_core::SaveFiles;
///
/// # fn main() -> Result<(), neoffse_core::FormatError> {
/// let mut files = SaveFiles::new();
/// files.push("slot.sav", vec![1, 2, 3])?;
/// assert_eq!(files.get("slot.sav"), Some(&[1, 2, 3][..]));
/// assert!(files.push("../slot.sav", Vec::new()).is_err());
/// # Ok(())
/// # }
/// ```
///
/// [`SaveFormat::open`]: crate::format::SaveFormat::open
/// [`SaveDocument::encode`]: crate::format::SaveDocument::encode
#[derive(Clone, Default, PartialEq, Eq)]
pub struct SaveFiles {
    files: Vec<(String, Vec<u8>)>,
}

impl SaveFiles {
    /// An empty set.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a file after the ones already present. An empty `bytes` is a
    /// file.
    ///
    /// A plain file name passes: it is not empty, not `.` or `..`, and holds
    /// no `/`, `\`, `:` or NUL byte. The rule keeps a name from leaving the
    /// save folder when a caller joins it to a directory. The backup request
    /// of `neoffse-io` is stricter: it also refuses `backup.json` in any ASCII
    /// case and two names equal under ASCII case folding. Here names are
    /// compared byte for byte: `A` and `a` are two files. A refused push
    /// changes nothing.
    ///
    /// # Errors
    ///
    /// `InvalidFileName` for a name that is not plain, `DuplicateFile` for a
    /// name already present.
    pub fn push(&mut self, name: impl Into<String>, bytes: Vec<u8>) -> Result<(), FormatError> {
        let name = name.into();
        if !is_plain_name(&name) {
            return Err(FormatError::InvalidFileName { name });
        }
        if self.files.iter().any(|(present, _)| *present == name) {
            return Err(FormatError::DuplicateFile { name });
        }
        self.files.push((name, bytes));
        Ok(())
    }

    /// The bytes of the file called `name`.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<&[u8]> {
        self.files.iter().find(|(present, _)| present == name).map(|(_, bytes)| bytes.as_slice())
    }

    /// The files in insertion order.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &[u8])> {
        self.files.iter().map(|(name, bytes)| (name.as_str(), bytes.as_slice()))
    }

    /// The number of files.
    #[must_use]
    pub fn len(&self) -> usize {
        self.files.len()
    }

    /// Whether the set holds no file.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.files.is_empty()
    }
}

fn is_plain_name(name: &str) -> bool {
    !name.is_empty() && name != "." && name != ".." && !name.contains(['/', '\\', ':', '\0'])
}

impl fmt::Debug for SaveFiles {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut list = f.debug_list();
        for (name, bytes) in &self.files {
            list.entry(&format_args!("{name:?}: {} bytes", bytes.len()));
        }
        list.finish()
    }
}
