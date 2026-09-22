/// Parses a Notchian-style world seed: a string that parses as an integer is used
/// literally, anything else (e.g. "Glacier") is hashed the way the vanilla client and
/// server hash a seed string.
pub fn parse_seed(input: &str) -> i64 {
    match input.parse::<i64>() {
        Ok(seed) => seed,
        Err(_) => java_string_hash(input) as i64,
    }
}

/// Java's `String.hashCode()`: `s[0]*31^(n-1) + ... + s[n-1]`, computed over UTF-16 code
/// units with 32-bit wraparound.
fn java_string_hash(s: &str) -> i32 {
    s.encode_utf16().fold(0i32, |hash, unit| hash.wrapping_mul(31).wrapping_add(unit as i32))
}
