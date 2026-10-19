//! The text layers of a file: BOM, line end and base64
//! (`docs/formats/pixel-remaster.md#bom-and-line-end`,
//! `docs/formats/pixel-remaster.md#base64`).

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;

use super::CodecError;

/// UTF-8 byte order mark (`docs/formats/pixel-remaster.md#bom-and-line-end`).
pub const BOM: [u8; 3] = [0xEF, 0xBB, 0xBF];

/// A file cut into its three framing parts. `bom + text + trailer` joins
/// back to the input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Framing<'a> {
    /// The file began with [`BOM`].
    pub bom: bool,
    /// The base64 text, whitespace inside it included.
    pub text: &'a [u8],
    /// The longest suffix made of space, tab, CR and LF.
    pub trailer: &'a [u8],
}

/// The four bytes the Go decoder trims after the text
/// (`docs/formats/pixel-remaster.md#bom-and-line-end`). Form feed and the
/// other `trim_ascii_end` bytes stay in the text.
pub(super) const fn is_trailer_byte(byte: u8) -> bool {
    matches!(byte, b' ' | b'\t' | b'\r' | b'\n')
}

/// Cuts `file` into BOM flag, base64 text and trailing whitespace.
///
/// The BOM counts only at byte 0.
#[must_use]
pub fn split(file: &[u8]) -> Framing<'_> {
    let (bom, rest) = match file.strip_prefix(&BOM) {
        Some(rest) => (true, rest),
        None => (false, file),
    };
    let text_len = rest.iter().rposition(|&byte| !is_trailer_byte(byte)).map_or(0, |index| index + 1);
    let (text, trailer) = rest.split_at_checked(text_len).unwrap_or((rest, &[]));
    Framing { bom, text, trailer }
}

/// Decodes the base64 text of a file.
///
/// `base_offset` is the position of `text` in the file (3 after a BOM) and
/// shifts the offsets of errors. A missing `=` is tolerated: the game pads,
/// the Go editor appends
/// (`docs/formats/pixel-remaster.md#base64`). Decoding stays strict
/// otherwise: standard alphabet, no embedded padding, zero trailing bits.
///
/// # Errors
///
/// [`CodecError::Base64`] when the text is not canonical base64.
pub fn decode_base64(text: &[u8], base_offset: usize) -> Result<Vec<u8>, CodecError> {
    let mut padded = text.to_vec();
    while !padded.len().is_multiple_of(4) {
        padded.push(b'=');
    }
    STANDARD.decode(&padded).map_err(|source| {
        let end = base_offset + text.len();
        let offset = match source {
            base64::DecodeError::InvalidByte(position, _) | base64::DecodeError::InvalidLastSymbol { offset: position, .. } => {
                base_offset + position.min(text.len())
            }
            _ => end,
        };
        CodecError::Base64 { offset, source }
    })
}

/// Encodes ciphertext as standard base64 with `=` padding: `4 * ceil(len /
/// 3)` characters (`docs/formats/pixel-remaster.md#base64`).
#[must_use]
pub fn encode_base64(cipher: &[u8]) -> String {
    STANDARD.encode(cipher)
}
