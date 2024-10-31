use std::fmt::Display;

/// Represents a Bencode value as defined in the BitTorrent specification.
///
/// Bencode (pronounced like B-encode) supports four different types of values:
/// - Byte strings (represented as [`String`])
/// - Integers
/// - Lists
/// - Dictionaries
#[derive(Debug, PartialEq)]
pub enum BValue {
    /// An integer value, can be positive or negative
    /// Example: `i42e` represents 42
    Integer(i64),

    /// A byte string, prefixed with its length
    /// Example: `4:spam` represents "spam"
    String(String),

    /// A list of BValue elements
    /// Example: `l4:spami42ee` represents ["spam", 42]
    List(Vec<BValue>),

    /// A dictionary mapping strings to BValues
    /// Example: `d3:bar4:spam3:fooi42ee` represents {"bar": "spam", "foo": 42}
    Dict(std::collections::BTreeMap<String, BValue>),
}

impl From<serde_json::Value> for BValue {
    fn from(value: serde_json::Value) -> Self {
        match value {
            serde_json::Value::Number(n) => BValue::Integer(n.as_i64().unwrap_or_default()),
            serde_json::Value::String(s) => BValue::String(s),
            serde_json::Value::Array(arr) => {
                BValue::List(arr.into_iter().map(BValue::from).collect())
            }
            serde_json::Value::Object(map) => {
                let btree = map.into_iter().map(|(k, v)| (k, BValue::from(v))).collect();
                BValue::Dict(btree)
            }
            _ => BValue::String("".to_string()),
        }
    }
}

impl From<BValue> for serde_json::Value {
    fn from(value: BValue) -> Self {
        match value {
            BValue::Integer(n) => serde_json::Value::Number(n.into()),
            BValue::String(s) => serde_json::Value::String(s),
            BValue::List(arr) => {
                serde_json::Value::Array(arr.into_iter().map(|v| v.into()).collect())
            }
            BValue::Dict(map) => {
                let obj = map.into_iter().map(|(k, v)| (k, v.into())).collect();
                serde_json::Value::Object(obj)
            }
        }
    }
}

impl Display for BValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BValue::Integer(n) => write!(f, "{}", n),
            // Requires quotes around string per the BitTorrent specification
            BValue::String(s) => write!(f, "\"{}\"", s),
            BValue::List(arr) => write!(
                f,
                "[{}]",
                arr.iter()
                    .map(|v| v.to_string())
                    .collect::<Vec<String>>()
                    .join(", ")
            ),
            BValue::Dict(map) => write!(
                f,
                "{}",
                map.iter()
                    .map(|(k, v)| format!("{}: {}", k, v))
                    .collect::<Vec<String>>()
                    .join(", ")
            ),
        }
    }
}