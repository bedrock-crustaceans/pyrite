use std::sync::LazyLock;

pub const MC_PI: f32 = std::f32::consts::PI;

// Built as a boxed slice via an iterator, not `Box::new(array::from_fn(...))` - the latter
// builds the full 256KB table as a stack temporary before moving it to the heap, which is large
// enough in a debug build to meaningfully eat into a caller's stack budget on the first call.
static SIN_TABLE: LazyLock<Box<[f32]>> = LazyLock::new(|| (0..65536).map(|i| (i as f64 * std::f64::consts::PI * 2.0 / 65536.0).sin() as f32).collect());

pub fn mc_sin(x: f32) -> f32 {
    let index = (x * 10430.378) as i32 as u16;
    SIN_TABLE[index as usize]
}

pub fn mc_cos(x: f32) -> f32 {
    let index = (x * 10430.378 + 16384.0) as i32 as u16;
    SIN_TABLE[index as usize]
}

pub fn mc_sin_cos(x: f32) -> (f32, f32) {
    (mc_sin(x), mc_cos(x))
}
