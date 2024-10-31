use anyhow::Result;
use bvalue::BValue;

mod bvalue;
mod decoder;
mod encoder;

/// Bencode decoder
#[derive(Debug, Clone, Copy)]
pub struct Bencode;

impl Bencode {
    /// Decode bencode string into a bvalue
    pub fn decode(input: &str) -> Result<BValue> {
        let value = decoder::Decoder::new(input).parse()?;
        Ok(value.into())
    }

    /// Encode plaintext to bencode string
    pub fn encode(value: &serde_json::Value) -> Result<String> {
        encoder::Encoder::new().encode(value)
    }
}
