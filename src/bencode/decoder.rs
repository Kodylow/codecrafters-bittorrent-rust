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
use tracing::{error, info};

use super::bvalue::BValue;

/// A streaming decoder for bencoded data.
///
/// The decoder maintains its position in the input string and parses values incrementally.
#[derive(Debug)]
pub struct Decoder<'a> {
    input: &'a [u8],
    position: usize,
}

impl<'a> Decoder<'a> {
    /// Creates a new decoder for the given input string.
    pub fn new(input: &'a str) -> Self {
        Self {
            input: input.as_bytes(),
            position: 0,
        }
    }

    /// Creates a new decoder for the given input bytes.
    pub fn new_from_bytes(input: &'a [u8]) -> Self {
        Self { input, position: 0 }
    }

    /// Parses the complete input string into a `BValue`.
    pub fn parse(&mut self) -> Result<BValue> {
        info!("parsing value");
        self.parse_value()
    }

    /// Returns the next byte in the input without consuming it.
    fn peek(&self) -> Option<u8> {
        self.input.get(self.position).copied()
    }

    /// Consumes and returns the next byte in the input.
    fn consume(&mut self) -> Option<u8> {
        let b = self.peek()?;
        self.position += 1;
        Some(b)
    }

    /// Consumes bytes until the given delimiter is found.
    /// Returns the consumed substring, excluding the delimiter.
    fn consume_until(&mut self, delimiter: u8) -> Result<&'a [u8]> {
        let start = self.position;
        while let Some(b) = self.peek() {
            if b == delimiter {
                let result = &self.input[start..self.position];
                self.consume(); // consume the delimiter
                return Ok(result);
            }
            self.consume();
        }
        Err(anyhow::anyhow!("Unexpected end of input"))
    }

    /// Parses a bencoded value based on its prefix:
    /// - 'i' for integers
    /// - 'l' for lists
    /// - 'd' for dictionaries
    /// - digit for strings
    fn parse_value(&mut self) -> Result<BValue> {
        info!("parsing value at position {}", self.position);
        match self.peek() {
            Some(b'i') => Ok(BValue::Integer(self.parse_integer()?)),
            Some(b'l') => self.parse_list(),
            Some(b'd') => self.parse_dict(),
            Some(c) if c.is_ascii_digit() => {
                // Direct conversion - no need to convert to String
                let bytes = self.parse_string()?;
                Ok(BValue::String(bytes))
            }
            Some(c) => {
                error!(
                    "Unhandled encoded value at position {}: {}",
                    self.position, c
                );
                Err(anyhow::anyhow!("Unhandled encoded value: {}", c))
            }
            None => Err(anyhow::anyhow!("Unexpected end of input")),
        }
    }

    /// Parses a bencoded integer of the form `i<number>e`.
    fn parse_integer(&mut self) -> Result<i64> {
        self.consume(); // consume 'i'
        let num_bytes = self.consume_until(b'e')?;
        std::str::from_utf8(num_bytes)
            .map(|num_str| num_str.parse::<i64>().map_err(Into::into))
            .map_err(|e| anyhow::anyhow!("Failed to parse integer: {}", e))?
    }

    /// Parses a bencoded string of the form `<length>:<contents>`.
    fn parse_string(&mut self) -> Result<Vec<u8>> {
        let len_bytes = self.consume_until(b':')?;
        let len = std::str::from_utf8(len_bytes)?.parse::<usize>()?;

        let start = self.position;
        for _ in 0..len {
            if self.consume().is_none() {
                return Err(anyhow::anyhow!("Unexpected end of string"));
            }
        }

        // Return raw bytes instead of converting to String
        Ok(self.input[start..self.position].to_vec())
    }

    /// Parses a bencoded list of the form `l<bencoded values>e`.
    fn parse_list(&mut self) -> Result<BValue> {
        self.consume(); // consume 'l'
        let mut values = Vec::new();

        while let Some(b) = self.peek() {
            if b == b'e' {
                self.consume();
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
        self.consume(); // consume 'd'
        let mut map = std::collections::BTreeMap::new();

        while let Some(b) = self.peek() {
            if b == b'e' {
                self.consume();
                return Ok(BValue::Dict(map));
            }

            let key = match self.parse_value() {
                Ok(val) => match val.into() {
                    BValue::String(s) => String::from_utf8(s)?,
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

    /// Decodes the parsed bencoded data to a JSON value.
    pub fn decode_to_json(&mut self) -> Result<serde_json::Value> {
        let bvalue = self.parse()?;
        Ok(bvalue.into())
    }

    /// Decodes the parsed bencoded data to a JSON value from bytes.
    pub fn decode_bytes_to_json(bytes: &'a [u8]) -> Result<serde_json::Value> {
        let mut decoder = Self::new_from_bytes(bytes);
        decoder.decode_to_json()
    }

    /// Decodes the parsed bencoded data to a JSON value from a string.
    pub fn decode_str_to_json(s: &'a str) -> Result<serde_json::Value> {
        let mut decoder = Self::new(s);
        decoder.decode_to_json()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_integer() {
        let mut decoder = Decoder::new("i42e");
        assert_eq!(decoder.parse().unwrap(), BValue::Integer(42));

        let mut decoder = Decoder::new("i-42e");
        assert_eq!(decoder.parse().unwrap(), BValue::Integer(-42));

        let mut decoder = Decoder::new("i0e");
        assert_eq!(decoder.parse().unwrap(), BValue::Integer(0));
    }

    #[test]
    fn test_parse_string() {
        let mut decoder = Decoder::new("4:spam");
        assert_eq!(
            decoder.parse().unwrap(),
            BValue::String("spam".as_bytes().to_vec())
        );

        let mut decoder = Decoder::new("0:");
        assert_eq!(
            decoder.parse().unwrap(),
            BValue::String("".as_bytes().to_vec())
        );

        let mut decoder = Decoder::new("13:Hello, World!");
        assert_eq!(
            decoder.parse().unwrap(),
            BValue::String("Hello, World!".as_bytes().to_vec())
        );
    }

    #[test]
    fn test_parse_list() {
        let mut decoder = Decoder::new("l4:spami42ee");
        assert_eq!(
            decoder.parse().unwrap(),
            BValue::List(vec![
                BValue::String("spam".as_bytes().to_vec()),
                BValue::Integer(42)
            ])
        );

        let mut decoder = Decoder::new("le");
        assert_eq!(decoder.parse().unwrap(), BValue::List(Vec::new()));

        let mut decoder = Decoder::new("li1ei2ei3ee");
        assert_eq!(
            decoder.parse().unwrap(),
            BValue::List(vec![
                BValue::Integer(1),
                BValue::Integer(2),
                BValue::Integer(3)
            ])
        );
    }

    #[test]
    fn test_parse_dict() {
        let mut decoder = Decoder::new("d3:bar4:spam3:fooi42ee");
        assert_eq!(
            decoder.parse().unwrap(),
            BValue::Dict(std::collections::BTreeMap::from([
                (
                    "bar".to_string(),
                    BValue::String("spam".as_bytes().to_vec())
                ),
                ("foo".to_string(), BValue::Integer(42))
            ]))
        );

        let mut decoder = Decoder::new("de");
        assert_eq!(
            decoder.parse().unwrap(),
            BValue::Dict(std::collections::BTreeMap::new())
        );
    }

    #[test]
    fn test_parse_nested() {
        let mut decoder = Decoder::new("d4:listl1:a1:b1:ce4:dictd1:x1:y1:zi42eee");
        assert_eq!(
            decoder.parse().unwrap(),
            BValue::Dict(std::collections::BTreeMap::from([
                (
                    "list".to_string(),
                    BValue::List(vec![
                        BValue::String("a".as_bytes().to_vec()),
                        BValue::String("b".as_bytes().to_vec()),
                        BValue::String("c".as_bytes().to_vec())
                    ])
                ),
                (
                    "dict".to_string(),
                    BValue::Dict(std::collections::BTreeMap::from([
                        ("x".to_string(), BValue::String("y".as_bytes().to_vec())),
                        ("z".to_string(), BValue::Integer(42))
                    ]))
                )
            ]))
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
