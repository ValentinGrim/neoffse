//! Final Fantasy I to VI Pixel Remaster saves: the codec that turns a PC
//! save file into its JSON text and back, a lossless document that reads and
//! edits that text by path, and, for FF I, the mapping of that document onto
//! the domain of `neoffse-core`.
//! `docs/formats/pixel-remaster.md` is the source of truth for every rule
//! this crate implements. The codec and the document move bytes and JSON
//! text and know no domain type; [`ff1`] is the one module that does, and it
//! knows only the FF I slot.
//!
//! - [`cipher`]: Rijndael with a 256-bit block, CBC mode, the PR key and IV,
//!   and the PR zero padding.
//! - [`container`]: the file around the JSON text (BOM, base64, cipher, raw
//!   deflate, CRLF), with `decode` and `encode`. Both cap the JSON text at
//!   [`MAX_PAYLOAD_LEN`](container::MAX_PAYLOAD_LEN).
//! - [`document`]: a lossless view of the JSON text, read and edited by path,
//!   with row operations on arrays.
//! - [`ff1`]: the FF I slot as fields of `neoffse-core`. [`ff1::Ff1Model`]
//!   reads and edits them through two interfaces, `apply` under a
//!   `CapPolicy` and `apply_game`, which edits the way the game does.
//!   [`ff1::Ff1Format`] and [`ff1::Ff1Document`] implement `SaveFormat` and
//!   `SaveDocument` on top of it.
//! - [`games`]: the six games with their game id, Steam app ID, folder under
//!   `My Games` and executable name, and [`games::identify`], which names the
//!   game of a slot document by its vehicle count.
//! - [`slots`]: the 22 slot file names shared by the six games, and the
//!   detection that recognizes a slot file by its name and head.
//!
//! Decode a file, edit one value, encode it again:
//!
//! ```
//! use neoffse_format_pr::container::{Container, decode, encode};
//! use neoffse_format_pr::document::{Document, Scalar, Segment};
//!
//! let json = r#"{"userData":"{\"gil\":100}"}"#;
//! let file = encode(json, &Container::game())?;
//!
//! let decoded = decode(&file)?;
//! let mut document = Document::parse(&decoded.json)?;
//! let gil = [Segment::Key("userData"), Segment::Doc, Segment::Key("gil")];
//! document.set(&gil, &Scalar::Int(250))?;
//!
//! let edited = encode(&document.to_json(), &decoded.container)?;
//! assert_eq!(decode(&edited)?.json, r#"{"userData":"{\"gil\":250}"}"#);
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

pub mod cipher;
pub mod container;
pub mod document;
pub mod ff1;
pub mod games;
mod open_error;
pub mod slots;
