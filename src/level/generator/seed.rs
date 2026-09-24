pub fn parse_seed(input: &str) -> i64 {
    input
        .parse::<i64>()
        .unwrap_or_else(|_| input.encode_utf16().fold(0i32, |hash, unit| i32::wrapping_add(hash.wrapping_mul(31), unit as i32)) as i64)
}
