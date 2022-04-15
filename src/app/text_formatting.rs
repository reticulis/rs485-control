use chrono::Timelike;

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

pub fn convert_text_to_hex(text: String) -> Result<Vec<u8>, &'static str> {
    if text.contains("+") {
        return Ok(text.chars().map(|c| c as u8).collect::<Vec<u8>>());
    }
    let text = text.split_whitespace().collect::<Vec<&str>>();
    let mut buf = Vec::new();
    for t in text {
        match u8::from_str_radix(t, 16) {
            Ok(i) => buf.push(i),
            Err(_) => return Err("Valid input!"),
        }
    }
    Ok(buf)
}