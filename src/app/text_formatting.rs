use chrono::Timelike;
use std::error::Error;

use super::values::TypeData;

pub fn convert_to_hex_format(buf: &[u8]) -> String {
    buf.iter().map(|&i| format!("0x{:>02X} ", i)).collect()
}

pub fn time_execute() -> String {
    let time = chrono::Local::now();
    format!(
        "{:>02}:{:>02}:{:>02}",
        time.hour(),
        time.minute(),
        time.second()
    )
}

pub fn convert_text_to_hex(text: String) -> Result<TypeData, Box<dyn Error>> {
    if text.contains('+') {
        return Ok(TypeData::ASCII(text.chars().map(|c| c as u8).collect()));
    }
    let mut vec = Vec::new();
    for t in text.split_whitespace() {
        match u8::from_str_radix(t, 16) {
            Ok(u) => vec.push(u),
            Err(e) => return Err(Box::new(e)),
        }
    }
    Ok(TypeData::MODBUS(vec))
}

pub fn convert_hex_to_ascii(vec: &[u8]) -> String {
    vec.iter().map(|&c| (c as char).to_string()).collect()
}
