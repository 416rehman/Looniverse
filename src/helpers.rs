/// Converts a byte array to a string of bytes
pub fn bytes_to_byte_string(str: &[u8]) -> String {
    let mut res = String::default();

    for c in str {
        res.push_str(&format!("{:b}", c));
    }

    res
}
