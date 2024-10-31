use anyhow::Result;
use tracing::error;

use super::bvalue::BValue;

pub struct Decoder<'a> {
    input: &'a str,
    position: usize,
}

impl<'a> Decoder<'a> {
    pub fn new(input: &'a str) -> Self {
        Self { input, position: 0 }
    }

    pub fn parse(&mut self) -> Result<serde_json::Value> {
        self.parse_value()
    }

    fn peek_char(&self) -> Option<char> {
        self.input[self.position..].chars().next()
    }

    fn consume_char(&mut self) -> Option<char> {
        let c = self.peek_char()?;
        self.position += c.len_utf8();
        Some(c)
    }

    fn consume_until(&mut self, delimiter: char) -> Result<&'a str> {
        let start = self.position;
        while let Some(c) = self.peek_char() {
            if c == delimiter {
                let result = &self.input[start..self.position];
                self.consume_char(); // consume the delimiter
                return Ok(result);
            }
            self.consume_char();
        }
        Err(anyhow::anyhow!("Unexpected end of input"))
    }

    fn parse_value(&mut self) -> Result<serde_json::Value> {
        let bvalue = match self.peek_char() {
            Some('i') => BValue::Integer(self.parse_integer()?),
            Some('l') => self.parse_list()?,
            Some('d') => self.parse_dict()?,
            Some(c) if c.is_digit(10) => BValue::String(self.parse_string()?),
            Some(c) => {
                error!(
                    "Unhandled encoded value at position {}: {}",
                    self.position, c
                );
                return Err(anyhow::anyhow!("Unhandled encoded value: {}", c));
            }
            None => return Err(anyhow::anyhow!("Unexpected end of input")),
        };
        Ok(bvalue.into())
    }

    fn parse_integer(&mut self) -> Result<i64> {
        self.consume_char(); // consume 'i'
        let num_str = self.consume_until('e')?;
        num_str.parse::<i64>().map_err(Into::into)
    }

    fn parse_string(&mut self) -> Result<String> {
        let len_str = self.consume_until(':')?;
        let len = len_str.parse::<usize>()?;

        let start = self.position;
        for _ in 0..len {
            self.consume_char()
                .ok_or(anyhow::anyhow!("String too short"))?;
        }
        let string = &self.input[start..self.position];
        Ok(string.to_string())
    }

    fn parse_list(&mut self) -> Result<BValue> {
        self.consume_char(); // consume 'l'
        let mut values = Vec::new();

        while let Some(c) = self.peek_char() {
            if c == 'e' {
                self.consume_char();
                return Ok(BValue::List(values));
            }
            let value: BValue = self.parse_value()?.into();
            values.push(value);
        }
        Err(anyhow::anyhow!("Unterminated list"))
    }

    fn parse_dict(&mut self) -> Result<BValue> {
        self.consume_char(); // consume 'd'
        let mut map = std::collections::BTreeMap::new();

        while let Some(c) = self.peek_char() {
            if c == 'e' {
                self.consume_char();
                return Ok(BValue::Dict(map));
            }
            let key = match self.parse_value()?.into() {
                BValue::String(s) => s,
                _ => return Err(anyhow::anyhow!("Dictionary key must be a string")),
            };
            let value: BValue = self.parse_value()?.into();
            map.insert(key, value);
        }
        Err(anyhow::anyhow!("Unterminated dictionary"))
    }
}
