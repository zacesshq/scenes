//! A small JSON reader, enough for the report bacon writes. It is written by hand so the crate
//! keeps its two dependencies.

use std::collections::BTreeMap;

/// A parsed JSON value. Numbers are kept as `f64`, which holds every count and exit code a
/// report carries exactly.
#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Array(Vec<Value>),
    Object(BTreeMap<String, Value>),
}

/// How deep arrays and objects may nest. A report nests five levels; the limit stops a
/// corrupt file from exhausting the stack.
const MAX_DEPTH: usize = 64;

impl Value {
    /// The value under `key` when this is an object.
    pub fn get(&self, key: &str) -> Option<&Value> {
        match self {
            Value::Object(map) => map.get(key),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::String(text) => Some(text),
            _ => None,
        }
    }

    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Value::Number(n) => Some(*n),
            _ => None,
        }
    }

    /// A whole number of zero or more, such as a count.
    pub fn as_u64(&self) -> Option<u64> {
        self.as_f64()
            .filter(|n| *n >= 0.0 && n.fract() == 0.0 && *n <= u64::MAX as f64)
            .map(|n| n as u64)
    }

    pub fn as_array(&self) -> Option<&[Value]> {
        match self {
            Value::Array(items) => Some(items),
            _ => None,
        }
    }

    pub fn is_null(&self) -> bool {
        matches!(self, Value::Null)
    }
}

/// Parses a whole JSON document.
pub fn parse(text: &str) -> Result<Value, String> {
    let mut parser = Parser {
        bytes: text.as_bytes(),
        at: 0,
    };
    let value = parser.value(0)?;
    parser.skip_space();
    if parser.at != parser.bytes.len() {
        return Err(parser.error("text after the end of the document"));
    }
    Ok(value)
}

struct Parser<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl Parser<'_> {
    fn error(&self, what: &str) -> String {
        format!("{what} at byte {}", self.at)
    }

    fn skip_space(&mut self) {
        while let Some(b' ' | b'\t' | b'\n' | b'\r') = self.bytes.get(self.at) {
            self.at += 1;
        }
    }

    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.at).copied()
    }

    fn expect(&mut self, byte: u8) -> Result<(), String> {
        if self.peek() == Some(byte) {
            self.at += 1;
            Ok(())
        } else {
            Err(self.error(&format!("expected '{}'", byte as char)))
        }
    }

    fn literal(&mut self, word: &str, value: Value) -> Result<Value, String> {
        if self.bytes[self.at..].starts_with(word.as_bytes()) {
            self.at += word.len();
            Ok(value)
        } else {
            Err(self.error("unknown word"))
        }
    }

    fn value(&mut self, depth: usize) -> Result<Value, String> {
        if depth > MAX_DEPTH {
            return Err(self.error("nested too deeply"));
        }
        self.skip_space();
        match self.peek() {
            Some(b'{') => self.object(depth),
            Some(b'[') => self.array(depth),
            Some(b'"') => self.string().map(Value::String),
            Some(b't') => self.literal("true", Value::Bool(true)),
            Some(b'f') => self.literal("false", Value::Bool(false)),
            Some(b'n') => self.literal("null", Value::Null),
            Some(b'-' | b'0'..=b'9') => self.number(),
            Some(_) => Err(self.error("unexpected character")),
            None => Err(self.error("unexpected end")),
        }
    }

    fn object(&mut self, depth: usize) -> Result<Value, String> {
        self.expect(b'{')?;
        let mut map = BTreeMap::new();
        self.skip_space();
        if self.peek() == Some(b'}') {
            self.at += 1;
            return Ok(Value::Object(map));
        }
        loop {
            self.skip_space();
            let key = self.string()?;
            self.skip_space();
            self.expect(b':')?;
            let value = self.value(depth + 1)?;
            map.insert(key, value);
            self.skip_space();
            match self.peek() {
                Some(b',') => self.at += 1,
                Some(b'}') => {
                    self.at += 1;
                    return Ok(Value::Object(map));
                }
                _ => return Err(self.error("expected ',' or '}'")),
            }
        }
    }

    fn array(&mut self, depth: usize) -> Result<Value, String> {
        self.expect(b'[')?;
        let mut items = Vec::new();
        self.skip_space();
        if self.peek() == Some(b']') {
            self.at += 1;
            return Ok(Value::Array(items));
        }
        loop {
            items.push(self.value(depth + 1)?);
            self.skip_space();
            match self.peek() {
                Some(b',') => self.at += 1,
                Some(b']') => {
                    self.at += 1;
                    return Ok(Value::Array(items));
                }
                _ => return Err(self.error("expected ',' or ']'")),
            }
        }
    }

    fn number(&mut self) -> Result<Value, String> {
        let start = self.at;
        while let Some(b'-' | b'+' | b'.' | b'e' | b'E' | b'0'..=b'9') = self.peek() {
            self.at += 1;
        }
        std::str::from_utf8(&self.bytes[start..self.at])
            .ok()
            .and_then(|text| text.parse::<f64>().ok())
            .filter(|n| n.is_finite())
            .map(Value::Number)
            .ok_or_else(|| self.error("bad number"))
    }

    fn hex4(&mut self) -> Result<u32, String> {
        let digits = self
            .bytes
            .get(self.at..self.at + 4)
            .and_then(|d| std::str::from_utf8(d).ok())
            .and_then(|d| u32::from_str_radix(d, 16).ok())
            .ok_or_else(|| self.error("bad \\u escape"))?;
        self.at += 4;
        Ok(digits)
    }

    fn string(&mut self) -> Result<String, String> {
        self.expect(b'"')?;
        let mut out = String::new();
        loop {
            let start = self.at;
            while let Some(byte) = self.peek() {
                if byte == b'"' || byte == b'\\' {
                    break;
                }
                self.at += 1;
            }
            // the input is a str, so a run of bytes between two ASCII marks is valid UTF-8
            out.push_str(
                std::str::from_utf8(&self.bytes[start..self.at])
                    .map_err(|_| self.error("bad text"))?,
            );
            match self.peek() {
                Some(b'"') => {
                    self.at += 1;
                    return Ok(out);
                }
                Some(b'\\') => {
                    self.at += 1;
                    let escape = self.peek().ok_or_else(|| self.error("unexpected end"))?;
                    self.at += 1;
                    match escape {
                        b'"' => out.push('"'),
                        b'\\' => out.push('\\'),
                        b'/' => out.push('/'),
                        b'b' => out.push('\u{8}'),
                        b'f' => out.push('\u{c}'),
                        b'n' => out.push('\n'),
                        b'r' => out.push('\r'),
                        b't' => out.push('\t'),
                        b'u' => {
                            let first = self.hex4()?;
                            let code = if (0xd800..0xdc00).contains(&first)
                                && self.bytes[self.at..].starts_with(b"\\u")
                            {
                                self.at += 2;
                                let second = self.hex4()?;
                                0x10000 + ((first - 0xd800) << 10) + (second.wrapping_sub(0xdc00))
                            } else {
                                first
                            };
                            out.push(char::from_u32(code).unwrap_or('\u{fffd}'));
                        }
                        _ => return Err(self.error("unknown escape")),
                    }
                }
                _ => return Err(self.error("unterminated string")),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_every_kind_of_value() {
        let value =
            parse(r#" {"a": [1, -2.5, 3e2], "b": {"c": null}, "d": true, "e": false, "f": "x"} "#)
                .unwrap();
        let numbers: Vec<f64> = value
            .get("a")
            .and_then(Value::as_array)
            .unwrap()
            .iter()
            .filter_map(Value::as_f64)
            .collect();
        assert_eq!(numbers, vec![1.0, -2.5, 300.0]);
        assert!(value.get("b").and_then(|b| b.get("c")).unwrap().is_null());
        assert_eq!(value.get("d"), Some(&Value::Bool(true)));
        assert_eq!(value.get("e"), Some(&Value::Bool(false)));
        assert_eq!(value.get("f").and_then(Value::as_str), Some("x"));
        assert_eq!(parse("[]").unwrap(), Value::Array(Vec::new()));
        assert_eq!(parse("{}").unwrap(), Value::Object(BTreeMap::new()));
    }

    #[test]
    fn decodes_escapes_and_unicode() {
        let value = parse(r#""a\"b\\c\n\u001b[1mé😀 é""#).unwrap();
        assert_eq!(value.as_str(), Some("a\"b\\c\n\u{1b}[1m\u{e9}\u{1f600} é"));
    }

    #[test]
    fn whole_numbers_convert_and_others_do_not() {
        assert_eq!(parse("101").unwrap().as_u64(), Some(101));
        assert_eq!(parse("-1").unwrap().as_u64(), None);
        assert_eq!(parse("1.5").unwrap().as_u64(), None);
    }

    #[test]
    fn rejects_broken_documents() {
        for text in [
            "",
            "{",
            "[1,",
            r#"{"a" 1}"#,
            r#""open"#,
            "tru",
            "[1] 2",
            r#""\x""#,
            "1e999",
        ] {
            assert!(parse(text).is_err(), "{text:?}");
        }
        let deep = "[".repeat(200) + &"]".repeat(200);
        assert!(parse(&deep).is_err());
    }
}
