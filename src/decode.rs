// use serde_bencode

pub fn decode_bencoded_value(bencoded_value: &str) -> serde_json::Value {
    // If encoded_value starts with a digit, it's a number
    if bencoded_value.chars().next().unwrap().is_digit(10) {
        // Example: "5:hello" -> "hello"
        let colon_index = bencoded_value.find(':').unwrap();
        let number_string = &bencoded_value[..colon_index];
        let number = number_string.parse::<i64>().unwrap();
        let string = &bencoded_value[colon_index + 1..colon_index + 1 + number as usize];
        return serde_json::Value::String(string.to_string());
    } else {
        panic!("Unhandled encoded value: {}", bencoded_value)
    }
}
