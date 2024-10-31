use anyhow::Result;
use tracing::error;

pub struct BencodeDecoder<'a> {
    input: &'a str,
    position: usize,
}

impl<'a> BencodeDecoder<'a> {
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
        match self.peek_char() {
            Some('i') => self.parse_integer(),
            Some('l') => self.parse_list(),
            Some('d') => self.parse_dict(),
            Some(c) if c.is_digit(10) => self.parse_string(),
            Some(c) => {
                error!(
                    "Unhandled encoded value at position {}: {}",
                    self.position, c
                );
                Err(anyhow::anyhow!("Unhandled encoded value: {}", c))
            }
            None => Err(anyhow::anyhow!("Unexpected end of input")),
        }
    }

    fn parse_integer(&mut self) -> Result<serde_json::Value> {
        self.consume_char(); // consume 'i'
        let num_str = self.consume_until('e')?;
        let number = num_str.parse::<i64>()?;
        Ok(serde_json::Value::Number(number.into()))
    }

    fn parse_string(&mut self) -> Result<serde_json::Value> {
        let len_str = self.consume_until(':')?;
        let len = len_str.parse::<usize>()?;

        let start = self.position;
        for _ in 0..len {
            self.consume_char()
                .ok_or(anyhow::anyhow!("String too short"))?;
        }
        let string = &self.input[start..self.position];

        Ok(serde_json::Value::String(string.to_string()))
    }

    fn parse_list(&mut self) -> Result<serde_json::Value> {
        self.consume_char(); // consume 'l'
        let mut values = Vec::new();

        while let Some(c) = self.peek_char() {
            if c == 'e' {
                self.consume_char();
                return Ok(serde_json::Value::Array(values));
            }
            values.push(self.parse_value()?);
        }
        Err(anyhow::anyhow!("Unterminated list"))
    }

    fn parse_dict(&mut self) -> Result<serde_json::Value> {
        self.consume_char(); // consume 'd'
        let mut map = serde_json::Map::new();

        while let Some(c) = self.peek_char() {
            if c == 'e' {
                self.consume_char();
                return Ok(serde_json::Value::Object(map));
            }
            let key = match self.parse_value()? {
                serde_json::Value::String(s) => s,
                _ => return Err(anyhow::anyhow!("Dictionary key must be a string")),
            };
            let value = self.parse_value()?;
            map.insert(key, value);
        }
        Err(anyhow::anyhow!("Unterminated dictionary"))
    }
}
