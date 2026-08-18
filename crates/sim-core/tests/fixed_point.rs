//! Q16.16 arithmetic and, more importantly, parsing decimals without a float.

use sim_core::fixed::{Fixed, ParseError};

#[test]
fn whole_numbers_round_trip() {
    for value in [0, 1, -1, 100, -100, 32_767, -32_768] {
        assert_eq!(Fixed::from_int(value).floor_to_int(), value);
    }
}

#[test]
fn parses_decimals_without_a_float() {
    // 0.5 and 0.25 are exact in binary; 0.27 and 0.1 are not, and must land on the
    // nearest representable value below rather than drifting.
    assert_eq!(Fixed::parse("1").unwrap(), Fixed::ONE);
    assert_eq!(Fixed::parse("0").unwrap(), Fixed::ZERO);
    assert_eq!(Fixed::parse("0.5").unwrap().raw(), 32_768);
    assert_eq!(Fixed::parse("0.25").unwrap().raw(), 16_384);
    assert_eq!(Fixed::parse("-0.5").unwrap().raw(), -32_768);
    assert_eq!(Fixed::parse("2.5").unwrap().raw(), 163_840);

    // 0.27 * 65536 = 17694.72, truncated to 17694.
    assert_eq!(Fixed::parse("0.27").unwrap().raw(), 17_694);
    // 0.15 * 65536 = 9830.4
    assert_eq!(Fixed::parse("0.15").unwrap().raw(), 9_830);
}

#[test]
fn parsing_is_exact_and_repeatable() {
    // The same text must give the same bits every time and everywhere — this is what a
    // float parse would not guarantee across platforms.
    for _ in 0..100 {
        assert_eq!(Fixed::parse("0.6").unwrap().raw(), 39_321);
    }
}

#[test]
fn rejects_malformed_input() {
    for text in [
        "", "-", ".", "1.", "1.2.3", "abc", "1 ", " 1", "0x10", "1e3", "1.0e3",
    ] {
        assert_eq!(
            Fixed::parse(text),
            Err(ParseError::Malformed),
            "{text:?} should not parse"
        );
    }
}

#[test]
fn rejects_values_outside_the_range() {
    assert_eq!(Fixed::parse("40000"), Err(ParseError::OutOfRange));
    assert_eq!(Fixed::parse("-40000"), Err(ParseError::OutOfRange));
}

#[test]
fn arithmetic_holds() {
    let half = Fixed::parse("0.5").unwrap();
    let quarter = Fixed::parse("0.25").unwrap();

    assert_eq!(half.saturating_add(half), Fixed::ONE);
    assert_eq!(half.saturating_sub(quarter), quarter);
    assert_eq!(half.saturating_mul(half), quarter);
    assert_eq!(Fixed::ONE.checked_div(Fixed::from_int(4)), Some(quarter));
    assert_eq!(Fixed::ONE.checked_div(Fixed::ZERO), None);
}

#[test]
fn saturates_instead_of_wrapping() {
    assert_eq!(Fixed::MAX.saturating_add(Fixed::ONE), Fixed::MAX);
    assert_eq!(Fixed::MIN.saturating_sub(Fixed::ONE), Fixed::MIN);
    assert_eq!(Fixed::from_int(i32::MAX), Fixed::MAX);
}

#[test]
fn displays_without_a_float() {
    assert_eq!(Fixed::ONE.to_string(), "1.0000");
    assert_eq!(Fixed::parse("0.5").unwrap().to_string(), "0.5000");
    assert_eq!(Fixed::parse("-2.25").unwrap().to_string(), "-2.2500");
}
