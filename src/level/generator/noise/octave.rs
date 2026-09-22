use glam::{DVec2, DVec3};

use crate::level::generator::java_rand::JavaRand;
use crate::level::generator::noise::perlin::PerlinNoise;

#[derive(Clone, Debug)]
pub struct OctaveNoise {
    octaves: Vec<PerlinNoise>,
}

impl OctaveNoise {
    pub fn new(rand: &mut JavaRand, octave_count: usize) -> Self {
        let octaves = (0..octave_count).map(|_| PerlinNoise::new(rand)).collect();
        Self { octaves }
    }

    pub fn sample_3d<const W: usize, const H: usize>(&self, grid: &mut [[[f64; W]; H]; W], offset: DVec3, scale: DVec3) {
        *grid = [[[0.0; W]; H]; W];
        let mut frequency = 1.0;
        for octave in &self.octaves {
            octave.add_3d(grid, offset, scale * frequency, 1.0 / frequency);
            frequency /= 2.0;
        }
    }

    pub fn sample_2d<const W: usize>(&self, grid: &mut [[f64; W]; W], offset: DVec2, scale: DVec2) {
        *grid = [[0.0; W]; W];
        let mut frequency = 1.0;
        for octave in &self.octaves {
            octave.add_2d(grid, offset, scale * frequency, 1.0 / frequency);
            frequency /= 2.0;
        }
    }

    pub fn sample_3d_slice<const W: usize>(&self, grid: &mut [[f64; W]; W], offset: DVec2, scale: f64) {
        *grid = [[0.0; W]; W];
        let mut frequency = 1.0;
        for octave in &self.octaves {
            octave.add_3d_slice(grid, offset, scale * frequency, 1.0 / frequency);
            frequency /= 2.0;
        }
    }

    pub fn sample_simplex_2d<const W: usize>(&self, grid: &mut [[f64; W]; W], offset: DVec2, scale: DVec2, frequency_factor: f64) {
        *grid = [[0.0; W]; W];
        let scale = scale / 1.5;
        let mut frequency = 1.0;
        let mut amplitude = 0.55;
        for octave in &self.octaves {
            octave.add_simplex_2d(grid, offset, scale * frequency, amplitude);
            frequency *= frequency_factor;
            amplitude *= 2.0;
        }
    }
}
