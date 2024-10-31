use crate::bencode::bvalue::BValue;
use anyhow::Result;

pub struct Encoder {
    output: String,
}

impl Encoder {
    pub fn new() -> Self {
        Self {
            output: String::new(),
        }
    }

    pub fn encode(&mut self, value: &serde_json::Value) -> Result<String> {
        let bvalue: BValue = value.clone().into();
        self.encode_value(&bvalue)?;
        Ok(self.output.clone())
    }

    fn encode_value(&mut self, value: &BValue) -> Result<()> {
        match value {
            BValue::Integer(n) => self.encode_integer(*n)?,
            BValue::String(s) => self.encode_string(s)?,
            BValue::List(list) => self.encode_list(list)?,
            BValue::Dict(dict) => self.encode_dict(dict)?,
        }
        Ok(())
    }

    fn encode_integer(&mut self, n: i64) -> Result<()> {
        self.output.push('i');
        self.output.push_str(&n.to_string());
        self.output.push('e');
        Ok(())
    }

    fn encode_string(&mut self, s: &str) -> Result<()> {
        self.output.push_str(&s.len().to_string());
        self.output.push(':');
        self.output.push_str(s);
        Ok(())
    }

    fn encode_list(&mut self, list: &[BValue]) -> Result<()> {
        self.output.push('l');
        for item in list {
            self.encode_value(item)?;
        }
        self.output.push('e');
        Ok(())
    }

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
