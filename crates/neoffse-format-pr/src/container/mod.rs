//! The Pixel Remaster file container: base64 text, Rijndael-256-CBC
//! ciphertext, raw DEFLATE stream, compact JSON
//! (`docs/formats/pixel-remaster.md#container-encoding`).
//!
//! [`decode`] turns the bytes of a PC save file into the JSON text and the
//! [`Container`] facts. [`encode`] turns them back into file bytes. The
//! stage modules [`text`] and [`deflate`] are public so tests can reach each
//! step. The codec works on byte slices and parses no JSON.
//!
//! The decrypted stream keeps its zero padding: no step strips it or
//! restores it, and the inflater stops at the final DEFLATE block, so the
//! padding after it does no harm. Only [`InflatePath::Fallback`] inflates a
//! view without the trailing zeros, for a stream that has no final block.

mod error;

pub mod deflate;
pub mod text;

pub use error::CodecError;

use crate::cipher::{BLOCK_SIZE, PR_IV, Rijndael256, pad_zeros};

/// Largest inflated payload [`decode`] accepts: 16 MiB. The measured
/// payloads run from 80 to 235 KB
/// (`docs/formats/pixel-remaster.md#raw-deflate`).
pub const MAX_PAYLOAD_LEN: usize = 16 * 1024 * 1024;

/// Largest raw DEFLATE stream [`decode`] expects: twice [`MAX_PAYLOAD_LEN`].
/// DEFLATE bounds a stream by no function of its payload (stored blocks may
/// be any length, empty ones included), so this is a margin: a stored-block
/// stream costs 5 bytes per block, about 0.02% of the payload at the 31 745
/// bytes per block flate2 writes at level 0.
const MAX_STREAM_LEN: usize = 2 * MAX_PAYLOAD_LEN;

/// Longest base64 text [`decode`] accepts, in bytes, BOM and trailer
/// excluded: about 43 MiB, the base64 form (4 characters per 3 bytes) of a
/// stream of twice [`MAX_PAYLOAD_LEN`] padded to 32 bytes.
///
/// It is a margin, not an exact bound. Any practical encoder stays under it
/// for a payload within [`MAX_PAYLOAD_LEN`], and [`decode`] refuses a longer
/// text before it decodes base64. The measured streams run from 6 to 84 KB
/// (`docs/formats/pixel-remaster.md#raw-deflate`).
pub const MAX_TEXT_LEN: usize = MAX_STREAM_LEN.next_multiple_of(BLOCK_SIZE).div_ceil(3) * 4;

/// Most zero bytes the Go `padder.Decode` strips, which the Fallback
/// mimics (`docs/formats/pixel-remaster.md#zero-padding-and-the-stripping-trap`).
const MAX_STRIP: usize = 31;

/// What [`decode`] learned about the file around the JSON text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Container {
    /// The file began with the UTF-8 BOM `EF BB BF`.
    pub bom: bool,
    /// The exact bytes after the base64 text: space, tab, CR, LF only.
    pub trailer: Vec<u8>,
}

impl Container {
    /// BOM and CRLF, the style the game writes
    /// (`docs/formats/pixel-remaster.md#bom-and-line-end`).
    #[must_use]
    pub fn game() -> Self {
        Self {
            bom: true,
            trailer: b"\r\n".to_vec(),
        }
    }
}

/// Which inflate path produced the payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InflatePath {
    /// A block with BFINAL set ended the stream (every measured file).
    FinalBlock,
    /// The Fallback of
    /// `docs/formats/pixel-remaster.md#zero-padding-and-the-stripping-trap`:
    /// the stream has no final block and ends where the input ends.
    Fallback,
}

/// The result of [`decode`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Decoded {
    /// The payload, compact JSON text. It is valid UTF-8; the codec checks
    /// no more than that.
    pub json: String,
    /// The framing around the payload.
    pub container: Container,
    /// Which inflate path recovered the payload.
    pub inflate_path: InflatePath,
}

/// Decodes the bytes of a PC save file.
///
/// The decrypted buffer keeps every byte: no unpadding, so a stream whose
/// last byte is zero survives
/// (`docs/formats/pixel-remaster.md#zero-padding-and-the-stripping-trap`).
/// The inflater stops at the final block and ignores the zero padding after
/// it. [`Container::trailer`] keeps the whitespace after the base64 text
/// byte for byte, with no cap on its length.
///
/// A base64 text longer than [`MAX_TEXT_LEN`] is refused before base64
/// decoding. The inflated payload may not exceed [`MAX_PAYLOAD_LEN`].
///
/// # Errors
///
/// A [`CodecError`] naming the failed step: [`CodecError::Empty`],
/// [`CodecError::Base64`], [`CodecError::CipherLength`], [`CodecError::Inflate`],
/// [`CodecError::InflateTruncated`], [`CodecError::PayloadTooLarge`] (an
/// inflated payload above [`MAX_PAYLOAD_LEN`], or a text above
/// [`MAX_TEXT_LEN`]) or [`CodecError::Utf8`].
///
/// # Examples
///
/// ```
/// use neoffse_format_pr::container::{Container, InflatePath, decode, encode};
///
/// let file = encode(r#"{"gil":100}"#, &Container::game())?;
/// let decoded = decode(&file)?;
/// assert_eq!(decoded.json, r#"{"gil":100}"#);
/// assert!(decoded.container.bom);
/// assert_eq!(decoded.container.trailer, b"\r\n");
/// assert_eq!(decoded.inflate_path, InflatePath::FinalBlock);
/// # Ok::<(), neoffse_format_pr::container::CodecError>(())
/// ```
pub fn decode(file: &[u8]) -> Result<Decoded, CodecError> {
    let framing = text::split(file);
    if framing.text.is_empty() {
        return Err(CodecError::Empty);
    }
    if framing.text.len() > MAX_TEXT_LEN {
        return Err(CodecError::PayloadTooLarge { limit: MAX_TEXT_LEN });
    }
    let base_offset = if framing.bom { text::BOM.len() } else { 0 };
    let mut plain = text::decode_base64(framing.text, base_offset)?;
    if plain.is_empty() {
        return Err(CodecError::CipherLength { len: 0 });
    }
    Rijndael256::pr().cbc_decrypt(&PR_IV, &mut plain)?;
    let (bytes, inflate_path) = inflate_plain(&plain)?;
    let json = String::from_utf8(bytes).map_err(|error| CodecError::Utf8 { source: error.utf8_error() })?;
    Ok(Decoded {
        json,
        container: Container {
            bom: framing.bom,
            trailer: framing.trailer.to_vec(),
        },
        inflate_path,
    })
}

/// Encodes JSON text into the bytes of a PC save file: compress, pad, encrypt,
/// base64, then the BOM and the trailer of `container`
/// (`docs/formats/pixel-remaster.md#pseudo-code`).
///
/// # Errors
///
/// [`CodecError::Deflate`] when compression fails, and the errors of [`seal`].
pub fn encode(json: &str, container: &Container) -> Result<Vec<u8>, CodecError> {
    let stream = deflate::deflate(json.as_bytes())?;
    seal(&stream, container)
}

/// Back half of [`encode`]: pads `stream` with zeros, encrypts it, writes
/// base64, then adds the BOM and the trailer.
///
/// Tests use it to wrap streams that [`deflate::deflate`] never produces
/// (sync-flushed, zero-ending, with a tail). An aligned stream gets no extra
/// block, as in the Go `padder.Encode`
/// (`docs/formats/pixel-remaster.md#zero-padding-and-the-stripping-trap`).
///
/// An empty stream gives a file with no base64 text, which [`decode`]
/// rejects as [`CodecError::Empty`]; [`encode`] never produces one because
/// DEFLATE output is never empty.
///
/// # Errors
///
/// [`CodecError::Trailer`] when the trailer holds a byte other than space,
/// tab, CR or LF: such a trailer would not decode again.
pub fn seal(stream: &[u8], container: &Container) -> Result<Vec<u8>, CodecError> {
    if let Some(index) = container.trailer.iter().position(|&byte| !text::is_trailer_byte(byte)) {
        return Err(CodecError::Trailer { index });
    }
    let mut cipher = stream.to_vec();
    pad_zeros(&mut cipher);
    Rijndael256::pr().cbc_encrypt(&PR_IV, &mut cipher)?;
    let encoded = text::encode_base64(&cipher);
    let mut file = Vec::with_capacity(text::BOM.len() + encoded.len() + container.trailer.len());
    if container.bom {
        file.extend_from_slice(&text::BOM);
    }
    file.extend_from_slice(encoded.as_bytes());
    file.extend_from_slice(&container.trailer);
    Ok(file)
}

/// Inflates the decrypted buffer, with the Fallback of
/// `docs/formats/pixel-remaster.md#zero-padding-and-the-stripping-trap`.
fn inflate_plain(plain: &[u8]) -> Result<(Vec<u8>, InflatePath), CodecError> {
    let primary = match deflate::inflate(plain, MAX_PAYLOAD_LEN) {
        Ok(done) if done.ended => return Ok((done.output, InflatePath::FinalBlock)),
        Ok(run_dry) => CodecError::InflateTruncated { consumed: run_dry.consumed },
        Err(error @ CodecError::PayloadTooLarge { .. }) => return Err(error),
        Err(error) => error,
    };
    let stripped = strip_zeros(plain);
    match deflate::inflate(stripped, MAX_PAYLOAD_LEN) {
        Ok(done) if !done.ended && done.consumed == stripped.len() && deflate::is_complete_object(&done.output) => Ok((done.output, InflatePath::Fallback)),
        _ => Err(primary),
    }
}

/// `plain` without its trailing zero bytes, at most [`MAX_STRIP`] of them:
/// the Go `padder.Decode` rule, applied to a copy-free view.
fn strip_zeros(plain: &[u8]) -> &[u8] {
    let (_, tail) = plain.split_at_checked(plain.len().saturating_sub(MAX_STRIP)).unwrap_or((plain, &[]));
    let zeros = tail.iter().rev().take_while(|&&byte| byte == 0).count();
    plain.get(..plain.len().saturating_sub(zeros)).unwrap_or(plain)
}
