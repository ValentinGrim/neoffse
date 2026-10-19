use crate::cipher::UnalignedLength;

/// A failed step of the container pipeline. The variant names the step.
///
/// The type derives `Debug` only: `flate2::DecompressError` and
/// `std::io::Error` are not `PartialEq`. Tests match with `matches!`.
#[derive(Debug, thiserror::Error)]
pub enum CodecError {
    /// The file holds no base64 text: it is empty, a lone BOM, or only
    /// whitespace.
    #[error("the file holds no base64 text")]
    Empty,
    /// The base64 text is damaged.
    #[error("invalid base64 near byte {offset}: {source}")]
    Base64 {
        /// Byte offset in the file, BOM included. For an invalid byte or an
        /// invalid last symbol it is the offending byte. For every other
        /// failure, and for a byte that the decoder reached only in the `=`
        /// the codec appended, it is the end of the base64 text.
        offset: usize,
        /// The decoder's error.
        source: base64::DecodeError,
    },
    /// The ciphertext is empty or not a multiple of 32 bytes. An
    /// [`UnalignedLength`] from the cipher converts to this variant.
    #[error("ciphertext length {len} is not a non-zero multiple of 32 bytes")]
    CipherLength {
        /// The ciphertext length in bytes.
        len: usize,
    },
    /// The DEFLATE stream holds invalid data.
    #[error("inflate failed after {offset} bytes of the stream: {source}")]
    Inflate {
        /// Bytes of the decrypted stream consumed when the error occurred. It
        /// indexes the plaintext, not the file.
        offset: usize,
        /// The inflater's error.
        source: flate2::DecompressError,
    },
    /// The DEFLATE stream ran out of input before a final block ended it, and
    /// the Fallback did not recover a complete object
    /// (`docs/formats/pixel-remaster.md#zero-padding-and-the-stripping-trap`).
    #[error("the DEFLATE stream ends after {consumed} bytes without a final block")]
    InflateTruncated {
        /// Bytes the inflater took before it ran dry.
        consumed: usize,
    },
    /// The inflated payload exceeds [`MAX_PAYLOAD_LEN`](super::MAX_PAYLOAD_LEN),
    /// or the base64 text exceeds [`MAX_TEXT_LEN`](super::MAX_TEXT_LEN) and
    /// cannot carry a payload within that cap. Both cases report the payload
    /// cap in `limit`.
    #[error("the payload exceeds {limit} bytes")]
    PayloadTooLarge {
        /// The limit in bytes.
        limit: usize,
    },
    /// The inflated payload is not UTF-8.
    #[error("the payload is not valid UTF-8: {source}")]
    Utf8 {
        /// The validation error, which holds the length of the valid prefix.
        source: std::str::Utf8Error,
    },
    /// The compressor failed to write.
    #[error("deflate failed: {source}")]
    Deflate {
        /// The I/O error of the compressor.
        source: std::io::Error,
    },
    /// The trailer holds a byte that a decoder would not trim.
    #[error("trailer byte {index} is not a space, tab, CR or LF")]
    Trailer {
        /// Index of the first offending byte in the trailer.
        index: usize,
    },
}

impl From<UnalignedLength> for CodecError {
    fn from(error: UnalignedLength) -> Self {
        Self::CipherLength { len: error.len }
    }
}
