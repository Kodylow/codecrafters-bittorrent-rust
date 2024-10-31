use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

mod bvalue;
mod decoder;
mod encoder;

/// Bencode decoder
#[derive(Debug, Clone, Copy)]
pub struct Bencode;

impl Bencode {
    /// Decode bencode string into any type that implements Deserialize
    pub fn decode_str<T: for<'de> Deserialize<'de>>(input: &str) -> Result<T> {
        let value = decoder::Decoder::new(input).parse()?;
        Ok(serde_json::from_value(value)?)
    }

    /// Decode bencode from file into any type that implements Deserialize
    pub fn decode_file<T: for<'de> Deserialize<'de>>(path: impl Into<PathBuf>) -> Result<T> {
        let contents = fs::read_to_string(path.into())?;
        Self::decode_str(&contents)
    }

    /// Encode any serializable type to bencode string
    pub fn encode_str<T: Serialize>(value: &T) -> Result<String> {
        let value = serde_json::to_value(value)?;
        encoder::Encoder::new().encode(&value)
    }

    /// Encode any serializable type to bencode file
    pub fn encode_file<T: Serialize>(value: &T, path: impl Into<PathBuf>) -> Result<()> {
        let encoded = Self::encode_str(value)?;
        fs::write(path.into(), encoded)?;
        Ok(())
    }
}
