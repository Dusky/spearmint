//! The hand-rolled JSON reader.
//!
//! It exists so element data never passes through an `f64` on its way into a sim that
//! spec 3.1 forbids from containing one. Numbers stay raw text until something asks
//! for them as an integer or as fixed-point.

use sim_core::json::{self, Json, JsonErrorKind};

#[test]
fn reads_the_shapes_the_data_file_uses() {
    let value = json::parse(r#"{"a": 1, "b": [true, false, null], "c": {"d": "text"}}"#).unwrap();

    assert_eq!(value.get("a").unwrap().as_i32(), Some(1));
    assert_eq!(
        value.get("b").unwrap().as_array().unwrap(),
        &[Json::Bool(true), Json::Bool(false), Json::Null]
    );
    assert_eq!(
        value.get("c").unwrap().get("d").unwrap().as_str(),
        Some("text")
    );
    assert!(value.get("missing").is_none());
}

/// The whole point: a number is a borrowed slice of the source, never a parsed float.
#[test]
fn numbers_stay_as_text() {
    let value = json::parse(r#"{"n": 0.30000000000000004}"#).unwrap();
    assert_eq!(
        value.get("n").unwrap().as_number_str(),
        Some("0.30000000000000004")
    );
    assert_eq!(value.get("n").unwrap().as_i32(), None, "not a whole number");
}

#[test]
fn preserves_member_order() {
    // Spec 3.1 forbids hash-map iteration in the sim; members keep source order.
    let value = json::parse(r#"{"z": 1, "a": 2, "m": 3}"#).unwrap();
    let Json::Object(members) = value else {
        panic!("expected an object")
    };
    let keys: Vec<&str> = members.iter().map(|(key, _)| *key).collect();
    assert_eq!(keys, ["z", "a", "m"]);
}

#[test]
fn handles_whitespace_and_empty_containers() {
    assert_eq!(json::parse("  {  }  ").unwrap(), Json::Object(vec![]));
    assert_eq!(json::parse("\n[\t]\r\n").unwrap(), Json::Array(vec![]));
}

#[test]
fn negative_and_zero_integers() {
    let value = json::parse(r#"[-1, 0, -2147483648, 2147483647]"#).unwrap();
    let items = value.as_array().unwrap();
    assert_eq!(items[0].as_i32(), Some(-1));
    assert_eq!(items[1].as_i32(), Some(0));
    assert_eq!(items[2].as_i32(), Some(i32::MIN));
    assert_eq!(items[3].as_i32(), Some(i32::MAX));
}

#[test]
fn rejects_what_it_does_not_support() {
    let cases = [
        (
            r#"{"a": "has \" escape"}"#,
            JsonErrorKind::EscapesUnsupported,
        ),
        (r#"{"a": 1e5}"#, JsonErrorKind::MalformedNumber),
        (r#"{"a": 01}"#, JsonErrorKind::MalformedNumber),
        (r#"{"a": "unterminated}"#, JsonErrorKind::UnterminatedString),
        (r#"{"a" 1}"#, JsonErrorKind::UnexpectedByte),
        (r#"{"a": 1"#, JsonErrorKind::UnexpectedEnd),
        (r#"{} extra"#, JsonErrorKind::TrailingData),
        (r#""#, JsonErrorKind::UnexpectedEnd),
    ];
    for (source, expected) in cases {
        let error = json::parse(source).expect_err(&format!("{source:?} should fail"));
        assert_eq!(error.kind, expected, "for {source:?}");
    }
}

#[test]
fn rejects_deep_nesting_instead_of_overflowing_the_stack() {
    let deep = format!("{}{}", "[".repeat(200), "]".repeat(200));
    let error = json::parse(&deep).expect_err("should refuse");
    assert_eq!(error.kind, JsonErrorKind::TooDeep);
}
