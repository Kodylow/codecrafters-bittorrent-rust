use std::fmt::Display;

#[derive(Debug, PartialEq)]
pub enum BValue {
    Integer(i64),
    String(String),
    List(Vec<BValue>),
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
            BValue::String(s) => write!(f, "{}", s),
            BValue::List(arr) => write!(f, "{:?}", arr),
            BValue::Dict(map) => write!(f, "{:?}", map),
        }
    }
}
