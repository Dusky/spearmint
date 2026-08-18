//! A minimal JSON reader, written rather than depended on.
//!
//! Every JSON crate parses numbers into `f64`. Spec 3.1 forbids floats anywhere in the
//! sim, so a dependency would quietly launder element data through a float on its way
//! in — the constraint violated at the data layer while the code above it looks clean.
//!
//! This reader never interprets numbers at all. It hands back the raw source slice and
//! lets `Fixed::parse` turn it into fixed-point with integer arithmetic.
//!
//! Two deliberate limits, both of which fail loudly rather than silently:
//! escape sequences in strings are rejected, and so is nesting past `MAX_DEPTH`.

use crate::fixed::Fixed;

/// Objects hold their members in source order in a `Vec`, not a map. Spec 3.1 forbids
/// hash-map iteration anywhere that could influence the sim, and this keeps element
/// loading order-stable by construction.
#[derive(Clone, Debug, PartialEq)]
pub enum Json<'a> {
    Null,
    Bool(bool),
    /// The raw source slice. Never parsed as a float.
    Number(&'a str),
    String(&'a str),
    Array(Vec<Json<'a>>),
    Object(Vec<(&'a str, Json<'a>)>),
}

const MAX_DEPTH: u32 = 32;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct JsonError {
    pub kind: JsonErrorKind,
    /// Byte offset into the source where the problem was found.
    pub offset: usize,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum JsonErrorKind {
    UnexpectedEnd,
    UnexpectedByte,
    /// A string contained a backslash. Supporting escapes would mean owning unescaped
    /// copies; rejecting them keeps every string a borrowed slice of the source.
    EscapesUnsupported,
    UnterminatedString,
    MalformedNumber,
    TooDeep,
    TrailingData,
}

pub fn parse(source: &str) -> Result<Json<'_>, JsonError> {
    let mut reader = Reader {
        bytes: source.as_bytes(),
        source,
        offset: 0,
    };
    reader.skip_whitespace();
    let value = reader.value(0)?;
    reader.skip_whitespace();
    if reader.offset != reader.bytes.len() {
        return Err(reader.error(JsonErrorKind::TrailingData));
    }
    Ok(value)
}

impl<'a> Json<'a> {
    /// Looks up an object member. `None` for a missing key or a non-object.
    pub fn get(&self, key: &str) -> Option<&Json<'a>> {
        match self {
            Json::Object(members) => members
                .iter()
                .find(|(name, _)| *name == key)
                .map(|(_, value)| value),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&'a str> {
        match self {
            Json::String(text) => Some(text),
            _ => None,
        }
    }

    pub fn as_array(&self) -> Option<&[Json<'a>]> {
        match self {
            Json::Array(items) => Some(items),
            _ => None,
        }
    }

    /// The raw number text, for callers that want to interpret it themselves.
    pub fn as_number_str(&self) -> Option<&'a str> {
        match self {
            Json::Number(text) => Some(text),
            _ => None,
        }
    }

    /// Whole numbers only — a fractional value here is a data error, not a rounding
    /// opportunity.
    pub fn as_i32(&self) -> Option<i32> {
        let text = self.as_number_str()?;
        let (negative, digits) = match text.strip_prefix('-') {
            Some(rest) => (true, rest),
            None => (false, text.strip_prefix('+').unwrap_or(text)),
        };
        if digits.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
            return None;
        }
        let mut value: i64 = 0;
        for byte in digits.bytes() {
            value = value.checked_mul(10)?.checked_add(i64::from(byte - b'0'))?;
            if value > i64::from(i32::MAX) + 1 {
                return None;
            }
        }
        let signed = if negative { -value } else { value };
        i32::try_from(signed).ok()
    }

    pub fn as_u8(&self) -> Option<u8> {
        u8::try_from(self.as_i32()?).ok()
    }

    /// Fixed-point, straight from the source text.
    pub fn as_fixed(&self) -> Option<Fixed> {
        Fixed::parse(self.as_number_str()?).ok()
    }
}

struct Reader<'a> {
    bytes: &'a [u8],
    source: &'a str,
    offset: usize,
}

impl<'a> Reader<'a> {
    fn error(&self, kind: JsonErrorKind) -> JsonError {
        JsonError {
            kind,
            offset: self.offset,
        }
    }

    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.offset).copied()
    }

    fn skip_whitespace(&mut self) {
        while let Some(byte) = self.peek() {
            if matches!(byte, b' ' | b'\t' | b'\r' | b'\n') {
                self.offset += 1;
            } else {
                break;
            }
        }
    }

    fn expect(&mut self, byte: u8) -> Result<(), JsonError> {
        match self.peek() {
            Some(found) if found == byte => {
                self.offset += 1;
                Ok(())
            }
            Some(_) => Err(self.error(JsonErrorKind::UnexpectedByte)),
            None => Err(self.error(JsonErrorKind::UnexpectedEnd)),
        }
    }

    fn literal(&mut self, word: &str) -> Result<(), JsonError> {
        if self.source[self.offset..].starts_with(word) {
            self.offset += word.len();
            Ok(())
        } else {
            Err(self.error(JsonErrorKind::UnexpectedByte))
        }
    }

    fn value(&mut self, depth: u32) -> Result<Json<'a>, JsonError> {
        if depth > MAX_DEPTH {
            return Err(self.error(JsonErrorKind::TooDeep));
        }
        match self
            .peek()
            .ok_or_else(|| self.error(JsonErrorKind::UnexpectedEnd))?
        {
            b'{' => self.object(depth),
            b'[' => self.array(depth),
            b'"' => self.string().map(Json::String),
            b't' => self.literal("true").map(|()| Json::Bool(true)),
            b'f' => self.literal("false").map(|()| Json::Bool(false)),
            b'n' => self.literal("null").map(|()| Json::Null),
            byte if byte == b'-' || byte.is_ascii_digit() => self.number(),
            _ => Err(self.error(JsonErrorKind::UnexpectedByte)),
        }
    }

    fn object(&mut self, depth: u32) -> Result<Json<'a>, JsonError> {
        self.expect(b'{')?;
        let mut members = Vec::new();
        self.skip_whitespace();
        if self.peek() == Some(b'}') {
            self.offset += 1;
            return Ok(Json::Object(members));
        }
        loop {
            self.skip_whitespace();
            let key = self.string()?;
            self.skip_whitespace();
            self.expect(b':')?;
            self.skip_whitespace();
            let value = self.value(depth + 1)?;
            members.push((key, value));
            self.skip_whitespace();
            match self.peek() {
                Some(b',') => self.offset += 1,
                Some(b'}') => {
                    self.offset += 1;
                    return Ok(Json::Object(members));
                }
                Some(_) => return Err(self.error(JsonErrorKind::UnexpectedByte)),
                None => return Err(self.error(JsonErrorKind::UnexpectedEnd)),
            }
        }
    }

    fn array(&mut self, depth: u32) -> Result<Json<'a>, JsonError> {
        self.expect(b'[')?;
        let mut items = Vec::new();
        self.skip_whitespace();
        if self.peek() == Some(b']') {
            self.offset += 1;
            return Ok(Json::Array(items));
        }
        loop {
            self.skip_whitespace();
            items.push(self.value(depth + 1)?);
            self.skip_whitespace();
            match self.peek() {
                Some(b',') => self.offset += 1,
                Some(b']') => {
                    self.offset += 1;
                    return Ok(Json::Array(items));
                }
                Some(_) => return Err(self.error(JsonErrorKind::UnexpectedByte)),
                None => return Err(self.error(JsonErrorKind::UnexpectedEnd)),
            }
        }
    }

    fn string(&mut self) -> Result<&'a str, JsonError> {
        self.expect(b'"')?;
        let start = self.offset;
        loop {
            match self.peek() {
                Some(b'"') => {
                    let text = &self.source[start..self.offset];
                    self.offset += 1;
                    return Ok(text);
                }
                Some(b'\\') => return Err(self.error(JsonErrorKind::EscapesUnsupported)),
                Some(_) => self.offset += 1,
                None => return Err(self.error(JsonErrorKind::UnterminatedString)),
            }
        }
    }

    fn number(&mut self) -> Result<Json<'a>, JsonError> {
        let start = self.offset;
        if self.peek() == Some(b'-') {
            self.offset += 1;
        }
        let digits_start = self.offset;
        while self.peek().is_some_and(|byte| byte.is_ascii_digit()) {
            self.offset += 1;
        }
        if self.offset == digits_start {
            return Err(self.error(JsonErrorKind::MalformedNumber));
        }
        // JSON forbids leading zeros, and in a hand-written data file `007` is a typo
        // rather than an intent. Rejecting beats silently reading it as 7.
        if self.offset - digits_start > 1 && self.bytes[digits_start] == b'0' {
            return Err(self.error(JsonErrorKind::MalformedNumber));
        }
        if self.peek() == Some(b'.') {
            self.offset += 1;
            let fraction_start = self.offset;
            while self.peek().is_some_and(|byte| byte.is_ascii_digit()) {
                self.offset += 1;
            }
            if self.offset == fraction_start {
                return Err(self.error(JsonErrorKind::MalformedNumber));
            }
        }
        // Exponents are rejected rather than parsed. Element data is hand-written, and
        // `1e3` there is likelier to be a mistake than an intent.
        if self.peek().is_some_and(|byte| byte == b'e' || byte == b'E') {
            return Err(self.error(JsonErrorKind::MalformedNumber));
        }
        Ok(Json::Number(&self.source[start..self.offset]))
    }
}
