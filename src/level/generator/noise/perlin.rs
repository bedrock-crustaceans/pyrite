use std::array;
use glam::{DVec2, DVec3};

use crate::rand::java::JavaRand;
use crate::rand::primitives::Bound;

#[derive(Clone, Debug)]
pub struct PerlinNoise {
    offset: DVec3,
    permutations: Box<[u16; 512]>,
}

impl PerlinNoise {
    pub fn new(rand: &mut JavaRand) -> Self {
        let offset = DVec3::new(
            rand.random::<f64>(),
            rand.random::<f64>(),
            rand.random::<f64>()
        ) * 256.0;

        let mut permutations = Box::new(array::from_fn::<u16, 512, _>(|i| if i < 256 { i as u16 } else { 0 }));

        for index in 0usize..256 {
            let bound = 256 - index as i32;
            let swap_with = rand.random_with::<i32>(Bound::new(bound)) as usize + index;
            permutations.swap(index, swap_with);
        }
        permutations.copy_within(0..256, 256);

        Self { offset, permutations }
    }

    /// Adds one octave of 3D noise into `grid`, indexed `[x][y][z]`.
    pub fn add_3d<const W: usize, const H: usize>(&self, grid: &mut [[[f64; W]; H]; W], offset: DVec3, scale: DVec3, amplitude: f64) {
        let mut last_y_index = usize::MAX;
        let (mut edge_y0z0, mut edge_y1z0, mut edge_y0z1, mut edge_y1z1) = (0.0, 0.0, 0.0, 0.0);

        for (grid_x, plane) in grid.iter_mut().enumerate() {
            let (x, x_fade, x_index) = axis_sample((offset.x + grid_x as f64) * scale.x + self.offset.x);
            for grid_z in 0..W {
                let (z, z_fade, z_index) = axis_sample((offset.z + grid_z as f64) * scale.z + self.offset.z);
                for (grid_y, row) in plane.iter_mut().enumerate() {
                    let (y, y_fade, y_index) = axis_sample((offset.y + grid_y as f64) * scale.y + self.offset.y);
                    if grid_y == 0 || y_index != last_y_index {
                        last_y_index = y_index;

                        let a = self.permutations[x_index] as usize + y_index;
                        let a0 = self.permutations[a] as usize + z_index;
                        let a1 = self.permutations[a + 1] as usize + z_index;
                        let b = self.permutations[x_index + 1] as usize + y_index;
                        let b0 = self.permutations[b] as usize + z_index;
                        let b1 = self.permutations[b + 1] as usize + z_index;

                        edge_y0z0 = lerp(x_fade, gradient(self.permutations[a0], x, y, z), gradient(self.permutations[b0], x - 1.0, y, z));
                        edge_y1z0 = lerp(x_fade, gradient(self.permutations[a1], x, y - 1.0, z), gradient(self.permutations[b1], x - 1.0, y - 1.0, z));
                        edge_y0z1 = lerp(x_fade, gradient(self.permutations[a0 + 1], x, y, z - 1.0), gradient(self.permutations[b0 + 1], x - 1.0, y, z - 1.0));
                        edge_y1z1 = lerp(
                            x_fade,
                            gradient(self.permutations[a1 + 1], x, y - 1.0, z - 1.0),
                            gradient(self.permutations[b1 + 1], x - 1.0, y - 1.0, z - 1.0),
                        );
                    }

                    let value = lerp(z_fade, lerp(y_fade, edge_y0z0, edge_y1z0), lerp(y_fade, edge_y0z1, edge_y1z1));
                    row[grid_z] += value * amplitude;
                }
            }
        }
    }

    /// Adds one octave of 2D noise into `grid`, indexed `[x][z]`. Numerically distinct
    /// from slicing `add_3d` at a fixed height: this walks a dedicated 2D lattice
    /// formula (see `gradient_flat`), not the 3D one restricted to one layer.
    pub fn add_2d<const X: usize, const Z: usize>(&self, grid: &mut [[f64; X]; Z], offset: DVec2, scale: DVec2, amplitude: f64) {
        for (grid_x, grid_x_val) in grid.iter_mut().enumerate() {
            let (x, x_fade, x_index) = axis_sample((offset.x + grid_x as f64) * scale.x + self.offset.x);
            for (grid_z, grid_z_val) in grid_x_val.iter_mut().enumerate() {
                let (z, z_fade, z_index) = axis_sample((offset.y + grid_z as f64) * scale.y + self.offset.z);

                let a = self.permutations[x_index] as usize;
                let a0 = self.permutations[a] as usize + z_index;
                let b = self.permutations[x_index + 1] as usize;
                let b0 = self.permutations[b] as usize + z_index;

                let value = lerp(
                    z_fade,
                    lerp(x_fade, gradient_flat(self.permutations[a0], x, z), gradient(self.permutations[b0], x - 1.0, 0.0, z)),
                    lerp(x_fade, gradient(self.permutations[a0 + 1], x, 0.0, z - 1.0), gradient(self.permutations[b0 + 1], x - 1.0, 0.0, z - 1.0)),
                );

                *grid_z_val += value * amplitude;
            }
        }
    }

    /// Adds one octave of 2D noise into `grid`, indexed `[x][z]`, by walking a single
    /// horizontal slice through the *3D* lattice with the vertical axis pinned to this
    /// noise's own offset. Numerically distinct from `add_2d`: the reference generator
    /// samples its sand and thickness fields this way and its gravel field through
    /// `add_2d`, and the two are not interchangeable.
    pub fn add_3d_slice<const W: usize>(&self, grid: &mut [[f64; W]; W], offset: DVec2, scale: f64, amplitude: f64) {
        let (z, z_fade, z_index) = axis_sample(self.offset.z);
        let mut last_y_index = usize::MAX;
        let (mut edge_y0z0, mut edge_y1z0, mut edge_y0z1, mut edge_y1z1) = (0.0, 0.0, 0.0, 0.0);

        for (grid_x, grid_x_val) in grid.iter_mut().enumerate() {
            let (x, x_fade, x_index) = axis_sample((offset.x + grid_x as f64) * scale + self.offset.x);
            for (grid_y, grid_y_val) in grid_x_val.iter_mut().enumerate() {
                let (y, y_fade, y_index) = axis_sample((offset.y + grid_y as f64) * scale + self.offset.y);

                // Same cell-reuse behavior as `add_3d` - see its comment.
                if grid_y == 0 || y_index != last_y_index {
                    last_y_index = y_index;

                    let a = self.permutations[x_index] as usize + y_index;
                    let a0 = self.permutations[a] as usize + z_index;
                    let a1 = self.permutations[a + 1] as usize + z_index;
                    let b = self.permutations[x_index + 1] as usize + y_index;
                    let b0 = self.permutations[b] as usize + z_index;
                    let b1 = self.permutations[b + 1] as usize + z_index;

                    edge_y0z0 = lerp(x_fade, gradient(self.permutations[a0], x, y, z), gradient(self.permutations[b0], x - 1.0, y, z));
                    edge_y1z0 = lerp(x_fade, gradient(self.permutations[a1], x, y - 1.0, z), gradient(self.permutations[b1], x - 1.0, y - 1.0, z));
                    edge_y0z1 = lerp(x_fade, gradient(self.permutations[a0 + 1], x, y, z - 1.0), gradient(self.permutations[b0 + 1], x - 1.0, y, z - 1.0));
                    edge_y1z1 = lerp(
                        x_fade,
                        gradient(self.permutations[a1 + 1], x, y - 1.0, z - 1.0),
                        gradient(self.permutations[b1 + 1], x - 1.0, y - 1.0, z - 1.0),
                    );
                }

                let value = lerp(z_fade, lerp(y_fade, edge_y0z0, edge_y1z0), lerp(y_fade, edge_y0z1, edge_y1z1));
                *grid_y_val += value * amplitude;
            }
        }
    }

    /// Skewed-lattice noise (2D simplex noise) into `grid`, indexed `[x][z]`. Used for
    /// the temperature, humidity and biome climate fields; reuses this noise's own
    /// permutation table and offset.
    pub fn add_simplex_2d<const W: usize>(&self, grid: &mut [[f64; W]; W], offset: DVec2, scale: DVec2, amplitude: f64) {
        const SQRT_3: f64 = 1.7320508075688772;
        const SKEW: f64 = 0.5 * (SQRT_3 - 1.0);
        const UNSKEW: f64 = (3.0 - SQRT_3) / 6.0;

        for (grid_x, grid_x_val) in grid.iter_mut().enumerate() {
            let x = (offset.x + grid_x as f64) * scale.x + self.offset.x;
            for (grid_z, grid_z_val) in grid_x_val.iter_mut().enumerate() {
                let z = (offset.y + grid_z as f64) * scale.y + self.offset.y;

                let skew = (x + z) * SKEW;
                let pos = x + skew;
                let cell_x = pos.floor() as i32;
                let pos = z + skew;
                let cell_z = pos.floor() as i32;

                let unskew = cell_x.wrapping_add(cell_z) as f64 * UNSKEW;
                let delta_x = x - (cell_x as f64 - unskew);
                let delta_z = z - (cell_z as f64 - unskew);

                let (mid_x, mid_z) = if delta_x > delta_z { (1, 0) } else { (0, 1) };

                let mid_delta_x = delta_x - mid_x as f64 + UNSKEW;
                let mid_delta_z = delta_z - mid_z as f64 + UNSKEW;
                let far_delta_x = delta_x - 1.0 + 2.0 * UNSKEW;
                let far_delta_z = delta_z - 1.0 + 2.0 * UNSKEW;

                let cell_x_index = (cell_x & 255) as usize;
                let cell_z_index = (cell_z & 255) as usize;

                let near_corner = self.permutations[cell_x_index + self.permutations[cell_z_index] as usize] % 12;
                let mid_corner = self.permutations[cell_x_index + mid_x + self.permutations[cell_z_index + mid_z] as usize] % 12;
                let far_corner = self.permutations[cell_x_index + 1 + self.permutations[cell_z_index + 1] as usize] % 12;

                let value = simplex_corner(delta_x, delta_z, near_corner as usize)
                    + simplex_corner(mid_delta_x, mid_delta_z, mid_corner as usize)
                    + simplex_corner(far_delta_x, far_delta_z, far_corner as usize);

                *grid_z_val += 70.0 * value * amplitude;
            }
        }
    }
}

#[inline(always)]
fn lerp(t: f64, from: f64, to: f64) -> f64 {
    (t * (to - from)) + from
}

#[inline(always)]
fn gradient(hash: u16, x: f64, y: f64, z: f64) -> f64 {
    match hash & 15 {
        0  =>  x + y,
        1  => -x + y,
        2  =>  x - y,
        3  => -x - y,
        4  =>  x + z,
        5  => -x + z,
        6  =>  x - z,
        7  => -x - z,
        8  =>  y + z,
        9  => -y + z,
        10 =>  y - z,
        11 => -y - z,
        12 =>  x + y,
        13 => -y + z,
        14 => -x + y,
        15 => -y - z,
        // SAFETY: mask guarantees a value between 0 and 15, which are all matched
        _ => unsafe { std::hint::unreachable_unchecked() },
    }
}

#[inline(always)]
fn gradient_flat(hash: u16, x: f64, z: f64) -> f64 {
    match hash & 15 {
        0  =>  x,
        2  =>  x,
        1  => -x,
        3  => -x,
        4  =>  x + z,
        5  => -x + z,
        6  =>  x - z,
        7  => -x - z,
        8  =>  z,
        9  =>  z,
        10 => -z,
        11 => -z,
        12 =>  x,
        13 =>  z,
        14 => -x,
        15 => -z,
        // SAFETY: mask guarantees a value between 0 and 15, which are all matched
        _ => unsafe { std::hint::unreachable_unchecked() },
    }
}

#[inline(always)]
fn axis_sample(pos: f64) -> (f64, f64, usize) {
    let floor = pos.floor();
    let frac = pos - floor;

    let fade_inner = (frac * 6.0) -15.0;
    let fade_mid = (frac * fade_inner) + 10.0;
    let fade = frac * frac * frac * fade_mid;

    let index = (floor as i32 & 255) as usize;

    (frac, fade, index)
}

#[inline(always)]
fn simplex_corner(delta_x: f64, delta_z: f64, corner: usize) -> f64 {
    const CORNER_VECTORS: [(f64, f64); 12] = [
        (1.0, 1.0), (-1.0, 1.0), (1.0, -1.0), (-1.0, -1.0),
        (1.0, 0.0), (-1.0, 0.0), (1.0, 0.0), (-1.0, 0.0),
        (0.0, 1.0), (0.0, -1.0), (0.0, 1.0), (0.0, -1.0),
    ];

    let falloff = 0.5 - (delta_x * delta_x) - (delta_z * delta_z);
    if falloff < 0.0 {
        0.0
    } else {
        let (gx, gz) = CORNER_VECTORS[corner];

        let dot = (delta_x * gx) + (delta_z * gz);
        let f2 = falloff * falloff;
        let f4 = f2 * f2;

        f4 * dot
    }
}
