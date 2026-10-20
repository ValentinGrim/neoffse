//! Rijndael with a 256-bit block and a 256-bit key (14 rounds) in CBC mode,
//! the cipher of the Pixel Remaster saves
//! (`docs/formats/pixel-remaster.md#cipher`).
//!
//! Provenance: a hand port of `KiameV`'s ffpr-save-cypher (Go, MIT). That
//! library is based on py3rijndael (Python, MIT, Meyti), whose origin is a
//! gist that ports Bram Cohen's Java reference code of 2001 (public
//! domain). The notices sit in `licenses/`. The port computes the S-box and
//! the lookup tables at compile time and expands the key schedule in
//! [`Rijndael256::new`]; the Go library embeds both.
//!
//! Rijndael with a 256-bit block is not AES: AES fixes the block at 128
//! bits (`docs/formats/pixel-remaster.md#known-pitfalls`, item 5).
//!
//! The cipher is not constant time: table lookups depend on the data. That
//! is acceptable because the key is public and fixed, and the crate edits
//! local files.
//!
//! Decryption returns every byte. The Go `Decrypt` strips up to 31 trailing
//! zeros and loses stream bytes
//! (`docs/formats/pixel-remaster.md#zero-padding-and-the-stripping-trap`);
//! this module has no such function.

mod block;
mod cbc;
mod tables;

use std::fmt;

pub use block::Rijndael256;

/// Block size in bytes: 256 bits.
pub const BLOCK_SIZE: usize = 32;

/// The Pixel Remaster key (`docs/formats/pixel-remaster.md#cipher`).
pub const PR_KEY: [u8; 32] = [
    0x61, 0x09, 0x07, 0x00, 0xb9, 0xc3, 0xb8, 0xb9, 0x42, 0x15, 0x99, 0xe7, 0x9c, 0xa5, 0x7b, 0x87, 0xbe, 0xa9, 0x32, 0xd3, 0x79, 0x7b, 0xad, 0x76, 0x63, 0xed,
    0x4d, 0xde, 0x0a, 0x94, 0x3c, 0xc5,
];

/// The Pixel Remaster CBC initialization vector
/// (`docs/formats/pixel-remaster.md#cipher`).
pub const PR_IV: [u8; 32] = [
    0x02, 0x24, 0xf9, 0x77, 0x1f, 0xdb, 0x6e, 0x0e, 0x3b, 0xd5, 0x08, 0xd7, 0xb7, 0x95, 0xbf, 0x2e, 0x0c, 0xbd, 0x17, 0x69, 0x42, 0x68, 0x0a, 0x63, 0x7b, 0x12,
    0xbc, 0x62, 0x73, 0xdb, 0x2e, 0xbb,
];

/// A CBC input whose length is not a multiple of [`BLOCK_SIZE`]. A later
/// version may add fields, and only the CBC methods build one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct UnalignedLength {
    /// The rejected length in bytes.
    pub len: usize,
}

impl fmt::Display for UnalignedLength {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "length {} is not a multiple of the {BLOCK_SIZE}-byte block size", self.len)
    }
}

impl std::error::Error for UnalignedLength {}

/// Appends `(32 - len % 32) % 32` zero bytes to `data`.
///
/// Aligned and empty input stay as they are. The function changes `data`
/// and nothing else, and the `Vec` may reallocate to make room. A `&mut Vec`
/// is a unique borrow, so no other slice sees the appended bytes, unlike the
/// Go `append` that wrote into a backing array the caller shared
/// (`docs/formats/pixel-remaster.md#known-pitfalls`, item 6).
///
/// ```
/// use neoffse_format_pr::cipher::pad_zeros;
///
/// let mut data = vec![1, 2, 3];
/// pad_zeros(&mut data);
/// assert_eq!(data.len(), 32);
/// assert!(data[3..].iter().all(|&byte| byte == 0));
/// ```
pub fn pad_zeros(data: &mut Vec<u8>) {
    let padding = (BLOCK_SIZE - data.len() % BLOCK_SIZE) % BLOCK_SIZE;
    data.resize(data.len() + padding, 0);
}
