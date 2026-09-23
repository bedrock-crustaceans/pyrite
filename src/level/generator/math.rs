use std::sync::LazyLock;

pub const MC_PI: f32 = std::f32::consts::PI;

static SIN_TABLE: LazyLock<Box<[f32; 65536]>> = LazyLock::new(|| {
    let mut table = Box::new([0.0f32; 65536]);
    for (i, entry) in table.iter_mut().enumerate() {
        *entry = (i as f64 * std::f64::consts::PI * 2.0 / 65536.0).sin() as f32;
    }
    table
});

fn sin_table(index: u16) -> f32 {
    SIN_TABLE[index as usize]
}

pub fn mc_sin(x: f32) -> f32 {
    sin_table((x * 10430.378) as i32 as u16)
}

pub fn mc_cos(x: f32) -> f32 {
    sin_table((x * 10430.378 + 16384.0) as i32 as u16)
}

pub fn mc_sin_cos(x: f32) -> (f32, f32) {
    (mc_sin(x), mc_cos(x))
}