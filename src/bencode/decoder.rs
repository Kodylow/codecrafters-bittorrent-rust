//! Bencode decoder implementation following the BitTorrent protocol specification.
//!
//! This module provides functionality to parse bencoded data as defined in the
//! [BitTorrent protocol specification](http://www.bittorrent.org/beps/bep_0003.html#bencoding).
//!
//! Bencode supports four data types:
//! - Byte strings: `<length>:<contents>` (e.g. `4:spam`)
//! - Integers: `i<number>e` (e.g. `i42e`)
//! - Lists: `l<bencoded values>e` (e.g. `l4:spami42ee`)
//! - Dictionaries: `d<bencoded string><bencoded value>e` (e.g. `d3:bar4:spam3:fooi42ee`)

use anyhow::Result;
use tracing::error;

use super::bvalue::BValue;

/// A streaming decoder for bencoded data.
///
/// The decoder maintains its position in the input string and parses values incrementally.
#[derive(Debug)]
pub struct Decoder<'a> {
    input: &'a str,
    position: usize,
}

impl<'a> Decoder<'a> {
    /// Creates a new decoder for the given input string.
    pub fn new(input: &'a str) -> Self {
        Self { input, position: 0 }
    }

    /// Parses the complete input string into a `serde_json::Value`.
    pub fn parse(&mut self) -> Result<serde_json::Value> {
        self.parse_value()
    }

    /// Returns the next character in the input without consuming it.
    fn peek_char(&self) -> Option<char> {
        self.input[self.position..].chars().next()
    }

    /// Consumes and returns the next character in the input.
    fn consume_char(&mut self) -> Option<char> {
        let c = self.peek_char()?;
        self.position += c.len_utf8();
        Some(c)
    }

    /// Consumes characters until the given delimiter is found.
    /// Returns the consumed substring, excluding the delimiter.
    fn consume_until(&mut self, delimiter: char) -> Result<&'a str> {
        let start = self.position;
        while let Some(c) = self.peek_char() {
            if c == delimiter {
                let result = &self.input[start..self.position];
                self.consume_char(); // consume the delimiter
                return Ok(result);
            }
            self.consume_char();
        }
        Err(anyhow::anyhow!("Unexpected end of input"))
    }

    /// Parses a bencoded value based on its prefix:
    /// - 'i' for integers
    /// - 'l' for lists
    /// - 'd' for dictionaries
    /// - digit for strings
    fn parse_value(&mut self) -> Result<serde_json::Value> {
        let bvalue = match self.peek_char() {
            Some('i') => BValue::Integer(self.parse_integer()?),
            Some('l') => self.parse_list()?,
            Some('d') => self.parse_dict()?,
            Some(c) if c.is_digit(10) => BValue::String(self.parse_string()?),
            Some(c) => {
                error!(
                    "Unhandled encoded value at position {}: {}",
                    self.position, c
                );
                return Err(anyhow::anyhow!("Unhandled encoded value: {}", c));
            }
            None => return Err(anyhow::anyhow!("Unexpected end of input")),
        };
        Ok(bvalue.into())
    }

    /// Parses a bencoded integer of the form `i<number>e`.
    fn parse_integer(&mut self) -> Result<i64> {
        self.consume_char(); // consume 'i'
        let num_str = self.consume_until('e')?;
        num_str.parse::<i64>().map_err(Into::into)
    }

    /// Parses a bencoded string of the form `<length>:<contents>`.
    fn parse_string(&mut self) -> Result<String> {
        let len_str = self.consume_until(':')?;
        let len = len_str.parse::<usize>()?;

        let start = self.position;
        for _ in 0..len {
            self.consume_char()
                .ok_or(anyhow::anyhow!("String too short"))?;
        }
        let string = &self.input[start..self.position];
        Ok(string.to_string())
    }

    /// Parses a bencoded list of the form `l<bencoded values>e`.
    fn parse_list(&mut self) -> Result<BValue> {
        self.consume_char(); // consume 'l'
        let mut values = Vec::new();

        while let Some(c) = self.peek_char() {
            if c == 'e' {
                self.consume_char();
                return Ok(BValue::List(values));
            }
            let value: BValue = self.parse_value()?.into();
            values.push(value);
        }
        Err(anyhow::anyhow!("Unterminated list"))
    }

    /// Parses a bencoded dictionary of the form `d<bencoded string><bencoded value>e`.
    /// Dictionary keys must be strings according to the specification.
    fn parse_dict(&mut self) -> Result<BValue> {
        self.consume_char(); // consume 'd'
        let mut map = std::collections::BTreeMap::new();

        while let Some(c) = self.peek_char() {
            if c == 'e' {
                self.consume_char();
                return Ok(BValue::Dict(map));
            }

            let key = match self.parse_value() {
                Ok(val) => match val.into() {
                    BValue::String(s) => s,
                    _ => return Err(anyhow::anyhow!("Dictionary key must be a string")),
                },
                Err(_) => return Err(anyhow::anyhow!("Unterminated dictionary")),
            };

            let value: BValue = match self.parse_value() {
                Ok(val) => val.into(),
                Err(_) => return Err(anyhow::anyhow!("Unterminated dictionary")),
            };

            map.insert(key, value);
        }
        Err(anyhow::anyhow!("Unterminated dictionary"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_parse_integer() {
        let mut decoder = Decoder::new("i42e");
        assert_eq!(decoder.parse().unwrap(), json!(42));

        let mut decoder = Decoder::new("i-42e");
        assert_eq!(decoder.parse().unwrap(), json!(-42));

        let mut decoder = Decoder::new("i0e");
        assert_eq!(decoder.parse().unwrap(), json!(0));
    }

    #[test]
    fn test_parse_string() {
        let mut decoder = Decoder::new("4:spam");
        assert_eq!(decoder.parse().unwrap(), json!("spam"));

        let mut decoder = Decoder::new("0:");
        assert_eq!(decoder.parse().unwrap(), json!(""));

        let mut decoder = Decoder::new("13:Hello, World!");
        assert_eq!(decoder.parse().unwrap(), json!("Hello, World!"));
    }

    #[test]
    fn test_parse_list() {
        let mut decoder = Decoder::new("l4:spami42ee");
        assert_eq!(decoder.parse().unwrap(), json!(["spam", 42]));

        let mut decoder = Decoder::new("le");
        assert_eq!(decoder.parse().unwrap(), json!([]));

        let mut decoder = Decoder::new("li1ei2ei3ee");
        assert_eq!(decoder.parse().unwrap(), json!([1, 2, 3]));
    }

    #[test]
    fn test_parse_dict() {
        let mut decoder = Decoder::new("d3:bar4:spam3:fooi42ee");
        assert_eq!(decoder.parse().unwrap(), json!({"bar": "spam", "foo": 42}));

        let mut decoder = Decoder::new("de");
        assert_eq!(decoder.parse().unwrap(), json!({}));
    }

    #[test]
    fn test_parse_nested() {
        let mut decoder = Decoder::new("d4:listl1:a1:b1:ce4:dictd1:x1:y1:zi42eee");
        assert_eq!(
            decoder.parse().unwrap(),
            json!({
                "list": ["a", "b", "c"],
                "dict": {
                    "x": "y",
                    "z": 42
                }
            })
        );
    }

    #[test]
    fn test_error_cases() {
        let cases = vec![
            ("i42", "Unexpected end of input"),
            ("4spam", "Unexpected end of input"),
            ("l1:a", "Unterminated list"),
            ("d1:a", "Unterminated dictionary"),
            ("d1:ai1e1:b", "Unterminated dictionary"),
            ("di1ei2ee", "Dictionary key must be a string"),
        ];

        for (input, expected_err) in cases {
            let mut decoder = Decoder::new(input);
            let err = decoder.parse().unwrap_err();
            assert!(
                err.to_string().contains(expected_err),
                "for input '{}', expected error containing '{}', got '{}'",
                input,
                expected_err,
                err
            );
        }
    }
}
