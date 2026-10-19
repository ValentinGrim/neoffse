//! Raw DEFLATE, the compression layer
//! (`docs/formats/pixel-remaster.md#raw-deflate`).

use std::io::Write as _;

use flate2::write::DeflateEncoder;
use flate2::{Compression, Decompress, FlushDecompress, Status};

use super::CodecError;

/// Compression level of [`deflate`]: 6, flate2's default and the level of the
/// Go stream the game accepted (`docs/formats/pixel-remaster.md#raw-deflate`).
pub const LEVEL: u32 = 6;

/// Size of the output buffer of one inflate call.
const CHUNK_LEN: usize = 16 * 1024;

/// The result of [`inflate`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Inflated {
    /// Every byte the stream produced before it ended or ran dry.
    pub output: Vec<u8>,
    /// Input bytes the inflater took. For a stream that ended, this is the
    /// stream length and the tail is not counted.
    pub consumed: usize,
    /// A block with BFINAL set ended the stream.
    pub ended: bool,
}

/// Compresses `payload` into a raw DEFLATE stream that ends with a block with
/// BFINAL set (`docs/formats/pixel-remaster.md#raw-deflate`; write rule 2 in
/// `docs/formats/pixel-remaster.md#write-rules`). No zlib or gzip header.
///
/// # Errors
///
/// [`CodecError::Deflate`] when the compressor fails to write.
pub fn deflate(payload: &[u8]) -> Result<Vec<u8>, CodecError> {
    let mut encoder = DeflateEncoder::new(Vec::new(), Compression::new(LEVEL));
    encoder.write_all(payload).map_err(|source| CodecError::Deflate { source })?;
    encoder.finish().map_err(|source| CodecError::Deflate { source })
}

fn to_usize(value: u64) -> usize {
    usize::try_from(value).unwrap_or(usize::MAX)
}

/// Inflates a raw DEFLATE stream and stops at its final block, ignoring any
/// bytes after it.
///
/// A stream that runs out of input without a final block is `Ok` with
/// `ended` false: the Fallback of
/// `docs/formats/pixel-remaster.md#zero-padding-and-the-stripping-trap` needs
/// that case. The limit is checked after each 16 KiB chunk, so a bomb stops
/// at about `limit` plus one chunk; an output of exactly `limit` bytes
/// passes.
///
/// # Errors
///
/// [`CodecError::Inflate`] when the stream holds invalid data, and
/// [`CodecError::PayloadTooLarge`] when the output exceeds `limit` bytes.
pub fn inflate(stream: &[u8], limit: usize) -> Result<Inflated, CodecError> {
    let mut inflater = Decompress::new(false);
    let mut chunk = vec![0_u8; CHUNK_LEN];
    let mut output = Vec::new();
    loop {
        let before_in = inflater.total_in();
        let before_out = inflater.total_out();
        let input = stream.get(to_usize(before_in)..).unwrap_or(&[]);
        let status = inflater
            .decompress(input, &mut chunk, FlushDecompress::None)
            .map_err(|source| CodecError::Inflate {
                offset: to_usize(inflater.total_in()),
                source,
            })?;
        let written = to_usize(inflater.total_out() - before_out);
        output.extend_from_slice(chunk.get(..written).unwrap_or(&[]));
        if output.len() > limit {
            return Err(CodecError::PayloadTooLarge { limit });
        }
        let consumed = to_usize(inflater.total_in());
        if status == Status::StreamEnd {
            return Ok(Inflated { output, consumed, ended: true });
        }
        if inflater.total_in() == before_in && inflater.total_out() == before_out {
            return Ok(Inflated {
                output,
                consumed,
                ended: false,
            });
        }
    }
}

/// Whether `bytes` read as one balanced JSON object: it starts with `{`,
/// brackets balance outside strings, the depth first reaches 0 at the last
/// byte, and no string stays open. Brackets of different kinds are not
/// matched; the JSON document parser validates for real.
pub(super) fn is_complete_object(bytes: &[u8]) -> bool {
    if bytes.first() != Some(&b'{') {
        return false;
    }
    let mut depth = 0_usize;
    let mut in_string = false;
    let mut escaped = false;
    for (index, &byte) in bytes.iter().enumerate() {
        if in_string {
            match (escaped, byte) {
                (true, _) => escaped = false,
                (false, b'\\') => escaped = true,
                (false, b'"') => in_string = false,
                _ => {}
            }
            continue;
        }
        match byte {
            b'"' => in_string = true,
            b'{' | b'[' => depth += 1,
            b'}' | b']' => {
                let Some(next) = depth.checked_sub(1) else { return false };
                depth = next;
                if depth == 0 && index + 1 != bytes.len() {
                    return false;
                }
            }
            _ => {}
        }
    }
    depth == 0 && !in_string
}
