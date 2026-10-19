//! Final Fantasy I to VI Pixel Remaster saves: the codec (base64,
//! Rijndael-256, raw deflate), the lossless JSON document, and the mapping
//! to the domain model. `docs/formats/pixel-remaster.md` is the source of
//! truth for every rule this crate implements.
//!
//! - [`cipher`]: Rijndael with a 256-bit block, CBC mode, the PR key and IV,
//!   and the PR zero padding.
//! - [`container`]: the file around the JSON text (BOM, base64, cipher, raw
//!   deflate, CRLF), with `decode` and `encode`.
//! - [`document`]: a lossless view of the JSON text, read and edited by path.

pub mod cipher;
pub mod container;
pub mod document;
