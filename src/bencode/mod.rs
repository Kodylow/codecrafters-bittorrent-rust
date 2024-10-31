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
        decoder::Decoder::new(input).parse()
    }

    /// Encode plaintext to bencode string
    pub fn encode(value: &serde_json::Value) -> Result<String> {
        encoder::Encoder::new().encode(value)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_encode_decode_roundtrip() {
        let test_cases = vec![
            json!(42),
            json!("Hello, World!"),
            json!(["a", "b", "c"]),
            json!({"x": "y", "z": 42}),
            json!({
                "list": ["a", "b", "c"],
                "dict": {"x": "y", "z": 42}
            }),
        ];

        for value in test_cases {
            let encoded = Bencode::encode(&value).unwrap();
            let decoded: serde_json::Value = Bencode::decode(&encoded).unwrap().into();
            assert_eq!(value, decoded);
        }
    }

    #[test]
    fn test_decode_encode_roundtrip() {
        let test_cases = vec!["i42e", "4:spam", "l4:spami42ee", "d3:bar4:spam3:fooi42ee"];

        for input in test_cases {
            let decoded = Bencode::decode(input).unwrap();
            let encoded = Bencode::encode(&decoded.into()).unwrap();
            assert_eq!(input, encoded);
        }
    }
}
