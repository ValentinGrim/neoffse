//! `value_for`: what typed text or a choice means for a field.

#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

use neoffse::core::{FieldMeta, NamedValue, Shape, Value};
use neoffse_app::EditProblem;
use neoffse_app::editor::{Input, value_for};

fn int(min: i64, max: i64) -> FieldMeta {
    FieldMeta::new("Value", Shape::Int { min, max })
}

fn text(typed: &str) -> Input {
    Input::Text(typed.to_owned())
}

fn parse(meta: &FieldMeta, typed: &str) -> Result<Value, EditProblem> {
    value_for(meta, &text(typed))
}

#[test]
fn integers_parse_trim_and_clamp() {
    let meta = int(i64::MIN, i64::MAX);
    assert_eq!(parse(&meta, " 12 "), Ok(Value::Int(12)));
    assert_eq!(parse(&meta, "+5"), Ok(Value::Int(5)));
    assert_eq!(parse(&meta, "\t-7\n"), Ok(Value::Int(-7)));
    assert_eq!(parse(&meta, "007"), Ok(Value::Int(7)));
    assert_eq!(parse(&int(0, 100), "-3"), Ok(Value::Int(0)));
    assert_eq!(parse(&int(0, 50), "99"), Ok(Value::Int(50)));
    assert_eq!(parse(&int(1, 50), "0"), Ok(Value::Int(1)));
}

#[test]
fn bad_text_is_not_a_number() {
    let meta = int(0, 100);
    for typed in ["", " ", "-", "+", "1_000", "1.5", "0x10", "\u{661}\u{662}", "12a", "--1", "1 2", "\u{ff11}"] {
        assert_eq!(parse(&meta, typed), Err(EditProblem::NotANumber), "text {typed:?}");
    }
}

#[test]
fn overflow_clamps_by_sign() {
    let meta = int(-10, 50);
    let nines = "9".repeat(30);
    assert_eq!(parse(&meta, &nines), Ok(Value::Int(50)));
    assert_eq!(parse(&meta, &format!("-{nines}")), Ok(Value::Int(-10)));
    assert_eq!(parse(&meta, &format!("+{nines}")), Ok(Value::Int(50)));
    assert_eq!(parse(&meta, "9223372036854775808"), Ok(Value::Int(50)));
    assert_eq!(parse(&meta, "-9223372036854775808"), Ok(Value::Int(-10)));
    let wide = int(i64::MIN, i64::MAX);
    assert_eq!(parse(&wide, "-9223372036854775808"), Ok(Value::Int(i64::MIN)));
    assert_eq!(parse(&wide, "9223372036854775807"), Ok(Value::Int(i64::MAX)));
}

#[test]
fn an_invalid_shape_clamps_nothing() {
    let meta = int(5, 1);
    assert_eq!(parse(&meta, "3"), Ok(Value::Int(3)));
    assert_eq!(parse(&meta, "-300"), Ok(Value::Int(-300)));
    assert_eq!(parse(&meta, &"9".repeat(30)), Ok(Value::Int(i64::MAX)));
}

#[test]
fn read_only_is_refused_first() {
    let mut meta = int(0, 100);
    meta.read_only = true;
    assert_eq!(parse(&meta, "abc"), Err(EditProblem::ReadOnly));
    assert_eq!(value_for(&meta, &Input::Toggle(true)), Err(EditProblem::ReadOnly));
    assert_eq!(value_for(&meta, &Input::Pick(1)), Err(EditProblem::ReadOnly));
}

#[test]
fn each_input_fits_its_shape() {
    let mut meta = int(0, 10);
    meta.names = vec![
        NamedValue {
            value: 0,
            name: "Off".to_owned(),
        },
        NamedValue {
            value: 4,
            name: "Four".to_owned(),
        },
    ];
    assert_eq!(value_for(&meta, &Input::Toggle(true)), Err(EditProblem::NotEditable));
    assert_eq!(value_for(&meta, &Input::Pick(7)), Ok(Value::Int(7)));
    assert_eq!(value_for(&meta, &Input::Pick(500)), Ok(Value::Int(500)));
    assert_eq!(value_for(&meta, &Input::Name(1)), Ok(Value::Int(4)));
    assert_eq!(value_for(&meta, &Input::Name(9)), Err(EditProblem::NotEditable));

    let flag = FieldMeta::new("Flag", Shape::Bool);
    assert_eq!(value_for(&flag, &Input::Pick(1)), Err(EditProblem::NotEditable));
    assert_eq!(value_for(&flag, &Input::Toggle(true)), Ok(Value::Bool(true)));
    assert_eq!(value_for(&flag, &text("1")), Err(EditProblem::NotEditable));
    assert_eq!(value_for(&flag, &Input::Name(0)), Err(EditProblem::NotEditable));

    let name = FieldMeta::new("Name", Shape::Text);
    assert_eq!(value_for(&name, &text("x y")), Ok(Value::Text("x y".to_owned())));
    assert_eq!(value_for(&name, &text("  padded  ")), Ok(Value::Text("  padded  ".to_owned())));
    assert_eq!(value_for(&name, &Input::Pick(1)), Err(EditProblem::NotEditable));
    assert_eq!(value_for(&name, &Input::Toggle(false)), Err(EditProblem::NotEditable));
}

#[test]
fn debug_prints_the_variant_and_never_the_content() {
    assert_eq!(format!("{:?}", text("secret")), "Input::Text");
    assert_eq!(format!("{:?}", Input::Pick(12345)), "Input::Pick");
    assert_eq!(format!("{:?}", Input::Toggle(true)), "Input::Toggle");
    assert_eq!(format!("{:?}", Input::Name(3)), "Input::Name");
}
