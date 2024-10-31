//! Bencode encoder implementation following the BitTorrent protocol specification.
//!
//! This module provides functionality to encode data into the Bencode format as defined in the
//! [BitTorrent protocol specification](http://www.bittorrent.org/beps/bep_0003.html#bencoding).
//!
//! The encoding rules are:
//! - Strings are length-prefixed base10 followed by a colon and the string
//! - Integers are 'i' followed by the number in base10 followed by 'e'
//! - Lists are 'l' followed by their elements followed by 'e'
//! - Dictionaries are 'd' followed by alternating keys and values followed by 'e'

use crate::bencode::bvalue::BValue;
use anyhow::Result;
use tracing::info;

/// An encoder for converting data into Bencode format.
///
/// The encoder maintains an internal buffer and provides methods to encode
/// different data types according to the Bencode specification.
pub struct Encoder {
    output: String,
}

impl Encoder {
    /// Creates a new encoder with an empty output buffer.
    pub fn new() -> Self {
        Self {
            output: String::new(),
        }
    }

    /// Encodes a JSON value into a Bencode string.
    ///
    /// # Arguments
    ///
    /// * `value` - The JSON value to encode
    ///
    /// # Returns
    ///
    /// The Bencode-encoded string wrapped in a `Result`
    pub fn encode(&mut self, value: &serde_json::Value) -> Result<String> {
        let bvalue: BValue = value.clone().into();
        self.encode_value(&bvalue)?;
        Ok(self.output.clone())
    }

    /// Encodes a BValue into the internal buffer.
    fn encode_value(&mut self, value: &BValue) -> Result<()> {
        info!("encoding value: {}", value);
        match value {
            BValue::Integer(n) => self.encode_integer(*n)?,
            BValue::String(s) => self.encode_string(s)?,
            BValue::List(list) => self.encode_list(list)?,
            BValue::Dict(dict) => self.encode_dict(dict)?,
        }
        Ok(())
    }

    /// Encodes an integer in the format: i<number>e
    fn encode_integer(&mut self, n: i64) -> Result<()> {
        info!("encoding integer: {}", n);
        self.output.push('i');
        self.output.push_str(&n.to_string());
        self.output.push('e');
        Ok(())
    }

    /// Encodes a string in the format: <length>:<string>
    fn encode_string(&mut self, s: &str) -> Result<()> {
        info!("encoding string: {}", s);
        self.output.push_str(&s.len().to_string());
        self.output.push(':');
        self.output.push_str(s);
        Ok(())
    }

    /// Encodes a list in the format: l<bencoded values>e
    fn encode_list(&mut self, list: &[BValue]) -> Result<()> {
        info!("encoding list: {}", list.len());
        self.output.push('l');
        for item in list {
            self.encode_value(item)?;
        }
        self.output.push('e');
        Ok(())
    }

    /// Encodes a dictionary in the format: d<bencoded string><bencoded value>e
    fn encode_dict(&mut self, dict: &std::collections::BTreeMap<String, BValue>) -> Result<()> {
        info!("encoding dict: {}", dict.len());
        self.output.push('d');
        for (key, value) in dict {
            self.encode_string(key)?;
            self.encode_value(value)?;
        }
        self.output.push('e');
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_encode_integer() {
        let mut encoder = Encoder::new();
        assert_eq!(encoder.encode(&json!(42)).unwrap(), "i42e");

        let mut encoder = Encoder::new();
        assert_eq!(encoder.encode(&json!(-42)).unwrap(), "i-42e");

        let mut encoder = Encoder::new();
        assert_eq!(encoder.encode(&json!(0)).unwrap(), "i0e");
    }

    #[test]
    fn test_encode_string() {
        let mut encoder = Encoder::new();
        assert_eq!(encoder.encode(&json!("spam")).unwrap(), "4:spam");

        let mut encoder = Encoder::new();
        assert_eq!(encoder.encode(&json!("")).unwrap(), "0:");

        let mut encoder = Encoder::new();
        assert_eq!(
            encoder.encode(&json!("Hello, World!")).unwrap(),
            "13:Hello, World!"
        );
    }

    #[test]
    fn test_encode_list() {
        let mut encoder = Encoder::new();
        assert_eq!(
            encoder.encode(&json!(["spam", 42])).unwrap(),
            "l4:spami42ee"
        );

        let mut encoder = Encoder::new();
        assert_eq!(encoder.encode(&json!([])).unwrap(), "le");

        let mut encoder = Encoder::new();
        assert_eq!(encoder.encode(&json!([1, 2, 3])).unwrap(), "li1ei2ei3ee");
    }

    #[test]
    fn test_encode_dict() {
        let mut encoder = Encoder::new();
        assert_eq!(
            encoder.encode(&json!({"bar": "spam", "foo": 42})).unwrap(),
            "d3:bar4:spam3:fooi42ee"
        );

        let mut encoder = Encoder::new();
        assert_eq!(encoder.encode(&json!({})).unwrap(), "de");
    }

    #[test]
    fn test_encode_nested() {
        let mut encoder = Encoder::new();
        let encoded = encoder
            .encode(&json!({
                "dict": {
                    "x": "y",
                    "z": 42
                },
                "list": ["a", "b", "c"]
            }))
            .unwrap();

        // Create a decoder to parse and verify the structure matches
        let mut decoder = crate::bencode::decoder::Decoder::new(&encoded);
        let decoded = decoder.parse().unwrap();

        assert_eq!(
            decoded,
            BValue::Dict(std::collections::BTreeMap::from([
                (
                    "dict".to_string(),
                    BValue::Dict(std::collections::BTreeMap::from([
                        ("x".to_string(), BValue::String("y".to_string())),
                        ("z".to_string(), BValue::Integer(42))
                    ]))
                ),
                (
                    "list".to_string(),
                    BValue::List(vec![
                        BValue::String("a".to_string()),
                        BValue::String("b".to_string()),
                        BValue::String("c".to_string())
                    ])
                )
            ]))
        );
    }
}
