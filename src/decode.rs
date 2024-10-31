use anyhow::Result;
use tracing::error;

pub fn decode_bencoded_value(bencoded_value: &str) -> Result<serde_json::Value> {
    let first_char = bencoded_value
        .chars()
        .next()
        .ok_or(anyhow::anyhow!("Bencoded length value is empty"))?;
    match first_char.is_digit(10) {
        true => {
            let colon_index = bencoded_value
                .find(':')
                .ok_or(anyhow::anyhow!("Bencoded length value is missing colon"))?;
            let number_string = &bencoded_value[..colon_index];
            let number = number_string.parse::<i64>()?;
            let string = &bencoded_value[colon_index + 1..colon_index + 1 + number as usize];
            Ok(serde_json::Value::String(string.to_string()))
        }
        false => {
            error!("Unhandled encoded value: {}", bencoded_value);
            Err(anyhow::anyhow!(
                "Unhandled encoded value: {}",
                bencoded_value
            ))
        }
    }
}
