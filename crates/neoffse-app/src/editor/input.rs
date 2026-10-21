//! What the user typed or chose, and the value it means.

use std::fmt;

use neoffse::core::{FieldMeta, Shape, Value};

use crate::EditProblem;

/// What the user typed or chose.
///
/// `Debug` prints the variant name only: a typed value is save content, so
/// never log it.
#[derive(Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Input {
    /// Text from a line edit: a name, or a number still to parse.
    Text(String),
    /// The state of a switch.
    Toggle(bool),
    /// The id of a picker entry.
    Pick(i64),
    /// The index of a named value in `FieldMeta::names`.
    Name(usize),
}

impl fmt::Debug for Input {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Input::Text(_) => "Input::Text",
            Input::Toggle(_) => "Input::Toggle",
            Input::Pick(_) => "Input::Pick",
            Input::Name(_) => "Input::Name",
        })
    }
}

/// The [`Value`] an input means for a field.
///
/// A read-only field refuses first. Text on a string field passes unchanged
/// (no trim). Text on an integer field parses as whole decimal digits with
/// one optional sign, then clamps into the storage bounds of the shape
/// (`docs/formats/pixel-remaster.md#bounds-in-the-master-data`); digits past
/// `i64` saturate by sign before the clamp. A shape with `min > max` clamps
/// nothing: the facade reports it. A pick passes unclamped: the facade checks
/// the table and the bounds. A name index maps through `FieldMeta::names`.
///
/// # Errors
///
/// The [`EditProblem`] to show: `ReadOnly`, `NotANumber` or `NotEditable`.
pub fn value_for(meta: &FieldMeta, input: &Input) -> Result<Value, EditProblem> {
    if meta.read_only {
        return Err(EditProblem::ReadOnly);
    }
    match (input, meta.shape) {
        (Input::Text(text), Shape::Text) => Ok(Value::Text(text.clone())),
        (Input::Text(text), Shape::Int { min, max }) => parse_integer(text).map(|number| Value::Int(clamp(number, min, max))),
        (Input::Toggle(on), Shape::Bool) => Ok(Value::Bool(*on)),
        (Input::Pick(id), Shape::Int { .. }) => Ok(Value::Int(*id)),
        (Input::Name(index), Shape::Int { .. }) => meta.names.get(*index).map(|named| Value::Int(named.value)).ok_or(EditProblem::NotEditable),
        _ => Err(EditProblem::NotEditable),
    }
}

/// Clamps into `[min, max]`, or returns `number` when the range is empty.
fn clamp(number: i64, min: i64, max: i64) -> i64 {
    if min > max { number } else { number.clamp(min, max) }
}

/// Parses ASCII whitespace, one optional sign, then ASCII digits only. Digits
/// past `i64` saturate by sign.
fn parse_integer(text: &str) -> Result<i64, EditProblem> {
    let trimmed = text.trim_ascii();
    let (negative, digits) = match trimmed.as_bytes().first() {
        Some(b'-') => (true, &trimmed[1..]),
        Some(b'+') => (false, &trimmed[1..]),
        _ => (false, trimmed),
    };
    if digits.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(EditProblem::NotANumber);
    }
    let magnitude = digits
        .bytes()
        .fold(0_u64, |acc, byte| acc.saturating_mul(10).saturating_add(u64::from(byte - b'0')));
    Ok(if negative {
        // `i64::MIN` has a magnitude one above `i64::MAX`, so the negation of a fitting magnitude cannot overflow.
        i64::try_from(magnitude).map_or(i64::MIN, |number| -number)
    } else {
        i64::try_from(magnitude).unwrap_or(i64::MAX)
    })
}
