use lazy_static::*;

fn load_key(name: &str) -> Vec<u8> {
    let raw = std::env::var(name).expect(name);

    raw.split(',')
        .map(|byte| {
            let trimmed = byte.trim();
            if trimmed.starts_with("0x") || trimmed.starts_with("0X") {
                u8::from_str_radix(&trimmed[2..], 16).unwrap()
            } else {
                trimmed.parse().unwrap()
            }
        })
        .collect()
}

lazy_static! {
    pub static ref EXHEADER_PUBLIC_KEY: Vec<u8> = load_key("EXHEADER_PUBLIC_KEY");
    pub static ref CFA_PUBLIC_KEY: Vec<u8> = load_key("CFA_PUBLIC_KEY");
    pub static ref SCRAMBLER: Vec<u8> = load_key("SCRAMBLER");
    pub static ref KEY_X: Vec<u8> = load_key("KEY_X");
}
