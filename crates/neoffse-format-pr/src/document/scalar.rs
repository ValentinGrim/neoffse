use serde_json::{Number, Value};

use super::{DocumentError, Segment, render_path};

/// A JSON scalar read from or written to a [`Document`](super::Document).
#[derive(Debug, Clone, PartialEq)]
pub enum Scalar {
    /// `null`.
    Null,
    /// `true` or `false`.
    Bool(bool),
    /// An integer without `.`, `e` or `E` that fits `i64`. The text `-0`
    /// reads as `Int(0)`; [`Document::raw`](super::Document::raw) keeps the
    /// sign.
    Int(i64),
    /// An integer without `.`, `e` or `E` above `i64::MAX` that fits `u64`.
    /// A `Uint` below that bound reads back as `Int`: after
    /// `set(Scalar::Uint(5))`, `get` returns `Int(5)`.
    Uint(u64),
    /// A number with `.`, `e` or `E`. The written form is lost: `58.0` reads
    /// as `58.0_f64`. [`Document::raw`](super::Document::raw) keeps the text.
    Float(f64),
    /// A string, unescaped.
    Text(String),
}

impl Scalar {
    /// Classifies the JSON text of one value found at `path`.
    pub(super) fn from_text(text: &str, path: &[Segment<'_>]) -> Result<Self, DocumentError> {
        match text.as_bytes().first() {
            Some(b'n') => Ok(Self::Null),
            Some(b't') => Ok(Self::Bool(true)),
            Some(b'f') => Ok(Self::Bool(false)),
            Some(b'"') => serde_json::from_str::<String>(text)
                .map(Self::Text)
                .map_err(|source| DocumentError::InvalidJson {
                    path: render_path(path),
                    source,
                }),
            Some(b'{' | b'[') => Err(DocumentError::WrongKind {
                path: render_path(path),
                expected: "scalar",
            }),
            _ => Self::from_number(text).ok_or_else(|| DocumentError::NumberOutOfRange { path: render_path(path) }),
        }
    }

    fn from_number(text: &str) -> Option<Self> {
        if text.contains(['.', 'e', 'E']) {
            return text.parse::<f64>().ok().filter(|float| float.is_finite()).map(Self::Float);
        }
        match text.parse::<i64>() {
            Ok(int) => Some(Self::Int(int)),
            Err(_) => text.parse::<u64>().ok().map(Self::Uint),
        }
    }

    /// The JSON text to store for this value
    /// (`docs/formats/pixel-remaster.md#write-rules`, rule 4).
    ///
    /// A float goes through `Number::from_f64`, which prints `58.0` for 58
    /// where `f64::to_string` prints `58`.
    pub(super) fn json_text(&self, path: &[Segment<'_>]) -> Result<String, DocumentError> {
        Ok(match self {
            Self::Null => "null".to_owned(),
            Self::Bool(flag) => flag.to_string(),
            Self::Int(int) => int.to_string(),
            Self::Uint(uint) => uint.to_string(),
            Self::Float(float) => Number::from_f64(*float)
                .ok_or_else(|| DocumentError::NonFiniteFloat { path: render_path(path) })?
                .to_string(),
            Self::Text(text) => string_token(text),
        })
    }
}

/// The JSON string token of `text`, escaped by `serde_json`: `"`, `\` and the
/// bytes 0x00 to 0x1F are escaped; `/` and non-ASCII stay raw
/// (`docs/formats/pixel-remaster.md#json-text`).
pub(crate) fn string_token(text: &str) -> String {
    Value::String(text.to_owned()).to_string()
}
