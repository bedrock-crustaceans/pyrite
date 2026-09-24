use std::array;
use std::sync::LazyLock;

pub const MC_PI: f32 = std::f32::consts::PI;

static SIN_TABLE: LazyLock<Box<[f32; 65536]>> = LazyLock::new(|| Box::new(array::from_fn(|i| (i as f64 * std::f64::consts::PI * 2.0 / 65536.0).sin() as f32)));

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
