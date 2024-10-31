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
        self.output.push('i');
        self.output.push_str(&n.to_string());
        self.output.push('e');
        Ok(())
    }

    /// Encodes a string in the format: <length>:<string>
    fn encode_string(&mut self, s: &str) -> Result<()> {
        self.output.push_str(&s.len().to_string());
        self.output.push(':');
        self.output.push_str(s);
        Ok(())
    }

    /// Encodes a list in the format: l<bencoded values>e
    fn encode_list(&mut self, list: &[BValue]) -> Result<()> {
        self.output.push('l');
        for item in list {
            self.encode_value(item)?;
        }
        self.output.push('e');
        Ok(())
    }

    /// Encodes a dictionary in the format: d<bencoded string><bencoded value>e
    fn encode_dict(&mut self, dict: &std::collections::BTreeMap<String, BValue>) -> Result<()> {
        self.output.push('d');
        for (key, value) in dict {
            self.encode_string(key)?;
            self.encode_value(value)?;
        }
        self.output.push('e');
        Ok(())
    }
}
