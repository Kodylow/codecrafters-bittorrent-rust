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
    String(Vec<u8>),

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
            serde_json::Value::String(s) => BValue::String(s.into_bytes()),
            serde_json::Value::Array(arr) => {
                BValue::List(arr.into_iter().map(BValue::from).collect())
            }
            serde_json::Value::Object(map) => {
                let btree = map.into_iter().map(|(k, v)| (k, BValue::from(v))).collect();
                BValue::Dict(btree)
            }
            _ => BValue::String(Vec::new()),
        }
    }
}

impl From<BValue> for serde_json::Value {
    fn from(value: BValue) -> Self {
        match value {
            BValue::Integer(n) => serde_json::Value::Number(n.into()),
            BValue::String(s) => {
                let string = String::from_utf8_lossy(&s).into_owned();
                serde_json::Value::String(string)
            }
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
            BValue::String(s) => {
                let string = String::from_utf8_lossy(s);
                write!(f, "\"{}\"", string)
            }
            BValue::List(list) => {
                write!(f, "[")?;
                for (i, item) in list.iter().enumerate() {
                    if i > 0 {
                        write!(f, ",")?;
                    }
                    write!(f, "{}", item)?;
                }
                write!(f, "]")
            }
            BValue::Dict(dict) => {
                write!(f, "{{")?;
                for (i, (key, value)) in dict.iter().enumerate() {
                    if i > 0 {
                        write!(f, ",")?;
                    }
                    write!(f, "\"{}\":{}", key, value)?;
                }
                write!(f, "}}")
            }
        }
    }
}
