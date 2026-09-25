mod phases;
mod plant;
mod population;
mod vein;

use chorus::registry::block_registry::BlockRegistry;
use glam::{DVec2, DVec3};

use crate::level::generator::noise::octave::OctaveNoise;
use crate::level::generator::overworld::biome::{Biome, biome_from_climate};
use crate::level::generator::shared::block_ids::BlockIds;
use crate::level::generator::shared::cave::CaveCarver;
use crate::level::generator::shared::chunk_buffer::ChunkBuffer;
use crate::level::generator::shared::{CAVE_RADIUS, CHUNK_HEIGHT, CHUNK_WIDTH, dungeon, lake, snow, spring, tree};
use crate::level::generator::shared::{ClimateSource, TerrainSource, chunk_seed};
use crate::rand::java::JavaRand;
use crate::rand::primitives::Bound;

// Cast through f32 to match the reference generator's precision exactly - identical
// climate noise to the overworld's, since the reference's `ChunkProviderSky` reads
// biomes from the same shared `WorldChunkManager` the overworld uses.
const CLIMATE_SCALE: f64 = 0.025f32 as f64;
const CLIMATE_FREQUENCY_FACTOR: f64 = 0.25;
const HUMIDITY_SCALE: f64 = 0.05f32 as f64;
const HUMIDITY_FREQUENCY_FACTOR: f64 = 1.0 / 3.0;
const BIOME_SCALE: f64 = 0.25;
const BIOME_FREQUENCY_FACTOR: f64 = 0.5882352941176471;

const DENSITY_GRID_SIZE: usize = 3;
const DENSITY_GRID_HEIGHT: usize = 33;

const INTERP_GRID_SIZE: usize = DENSITY_GRID_SIZE - 1;
const INTERP_GRID_HEIGHT: usize = DENSITY_GRID_HEIGHT - 1;
const INTERP_STRIDE_XZ: usize = CHUNK_WIDTH / INTERP_GRID_SIZE;
const INTERP_STRIDE_Y: usize = CHUNK_HEIGHT / INTERP_GRID_HEIGHT;

const FADE_TARGET_DENSITY: f64 = -30.0;
const TOP_FADE_WIDTH: i32 = 32;
const BOTTOM_FADE_WIDTH: i32 = 8;

type ClimateField = [[f64; CHUNK_WIDTH]; CHUNK_WIDTH];
type SurfaceField = [[f64; CHUNK_WIDTH]; CHUNK_WIDTH];
type BiomeGrid = [[Biome; CHUNK_WIDTH]; CHUNK_WIDTH];
type DensityField = [[[f64; DENSITY_GRID_SIZE]; DENSITY_GRID_HEIGHT]; DENSITY_GRID_SIZE];

pub struct SkyGenerator {
    pub seed: i64,
    block_ids: BlockIds,

    temperature_noise: OctaveNoise,
    humidity_noise: OctaveNoise,
    biome_noise: OctaveNoise,

    low_noise: OctaveNoise,
    high_noise: OctaveNoise,
    blend_noise: OctaveNoise,

    #[allow(dead_code)]
    sand_gravel_noise: OctaveNoise,
    thickness_noise: OctaveNoise,

    #[allow(dead_code)]
    island_a_noise: OctaveNoise,
    #[allow(dead_code)]
    island_b_noise: OctaveNoise,

    feature_noise: OctaveNoise,
}

impl SkyGenerator {
    pub fn new(seed: i64, registry: &BlockRegistry) -> Self {
        let mut rand = JavaRand::new(seed);

        Self {
            seed,
            block_ids: BlockIds::resolve(registry),

            temperature_noise: OctaveNoise::new(&mut JavaRand::new(seed.wrapping_mul(9871)), 4),
            humidity_noise: OctaveNoise::new(&mut JavaRand::new(seed.wrapping_mul(39811)), 4),
            biome_noise: OctaveNoise::new(&mut JavaRand::new(seed.wrapping_mul(543321)), 2),

            // Construction order matters (each draws from the same `rand`) - matches
            // `ChunkProviderSky`'s own field order exactly. Two of its 8 fields (a 10-
            // and a 16-octave "island" pair, sampled but never actually read - traced
            // through `generateTerrain`'s `var29`/`var36`, both dead) are still
            // constructed here even though their sampled output is never used, since
            // `feature_noise` (tree-count noise) is constructed after them and needs
            // the RNG left in the right place.
            low_noise: OctaveNoise::new(&mut rand, 16),
            high_noise: OctaveNoise::new(&mut rand, 16),
            blend_noise: OctaveNoise::new(&mut rand, 8),
            sand_gravel_noise: OctaveNoise::new(&mut rand, 4),
            thickness_noise: OctaveNoise::new(&mut rand, 4),
            island_a_noise: OctaveNoise::new(&mut rand, 10),
            island_b_noise: OctaveNoise::new(&mut rand, 16),
            feature_noise: OctaveNoise::new(&mut rand, 8),
        }
    }

    fn generate_biomes(&self, x: i32, z: i32, temperature_grid: &mut ClimateField, humidity_grid: &mut ClimateField) -> BiomeGrid {
        let world_offset = DVec2::new((x * CHUNK_WIDTH as i32) as f64, (z * CHUNK_WIDTH as i32) as f64);

        let mut biome_noise_grid: ClimateField = [[0.0; CHUNK_WIDTH]; CHUNK_WIDTH];

        self.temperature_noise
            .sample_simplex_2d(temperature_grid, world_offset, DVec2::splat(CLIMATE_SCALE), CLIMATE_FREQUENCY_FACTOR);
        self.humidity_noise
            .sample_simplex_2d(humidity_grid, world_offset, DVec2::splat(HUMIDITY_SCALE), HUMIDITY_FREQUENCY_FACTOR);
        self.biome_noise
            .sample_simplex_2d(&mut biome_noise_grid, world_offset, DVec2::splat(BIOME_SCALE), BIOME_FREQUENCY_FACTOR);

        let mut biome_grid = [[Biome::Void; CHUNK_WIDTH]; CHUNK_WIDTH];

        for lx in 0..CHUNK_WIDTH {
            for lz in 0..CHUNK_WIDTH {
                let (temperature, humidity, biome) = classify_climate(temperature_grid[lx][lz], humidity_grid[lx][lz], biome_noise_grid[lx][lz]);

                temperature_grid[lx][lz] = temperature;
                humidity_grid[lx][lz] = humidity;
                biome_grid[lx][lz] = biome;
            }
        }

        biome_grid
    }

    fn build_terrain_column(&self, x: i32, z: i32) -> ChunkBuffer {
        let block_ids = &self.block_ids;
        let mut column = ChunkBuffer::new(block_ids.air);

        let mut temperature_grid: ClimateField = [[0.0; CHUNK_WIDTH]; CHUNK_WIDTH];
        let mut humidity_grid: ClimateField = [[0.0; CHUNK_WIDTH]; CHUNK_WIDTH];
        let biome_grid = self.generate_biomes(x, z, &mut temperature_grid, &mut humidity_grid);

        let density = self.build_density_field(x, z);
        place_terrain(&mut column, &density, block_ids);

        let mut rand = JavaRand::new(i64::wrapping_add((x as i64).wrapping_mul(341873128712), (z as i64).wrapping_mul(132897987541)));
        self.generate_surface(x, z, &mut column, &biome_grid, &mut rand, block_ids);

        column
    }

    fn climate_at(&self, x: i32, z: i32) -> (f64, f64, Biome) {
        let offset = DVec2::new(x as f64, z as f64);

        let mut temperature = [[0.0f64; 1]; 1];
        let mut humidity = [[0.0f64; 1]; 1];
        let mut biome_noise = [[0.0f64; 1]; 1];

        self.temperature_noise
            .sample_simplex_2d(&mut temperature, offset, DVec2::splat(CLIMATE_SCALE), CLIMATE_FREQUENCY_FACTOR);
        self.humidity_noise.sample_simplex_2d(&mut humidity, offset, DVec2::splat(HUMIDITY_SCALE), HUMIDITY_FREQUENCY_FACTOR);
        self.biome_noise.sample_simplex_2d(&mut biome_noise, offset, DVec2::splat(BIOME_SCALE), BIOME_FREQUENCY_FACTOR);

        classify_climate(temperature[0][0], humidity[0][0], biome_noise[0][0])
    }

    fn biome_at(&self, x: i32, z: i32) -> Biome {
        self.climate_at(x, z).2
    }

    fn feature_noise_at(&self, x: f64, z: f64) -> f64 {
        let mut value = [[[0.0f64; 1]; 1]; 1];
        self.feature_noise.sample_3d(&mut value, DVec3::new(x, z, 0.0), DVec3::ONE);
        value[0][0][0]
    }

    fn build_density_field(&self, x: i32, z: i32) -> DensityField {
        let world_offset_2d = DVec2::new((x * INTERP_GRID_SIZE as i32) as f64, (z * INTERP_GRID_SIZE as i32) as f64);
        let world_offset_3d = DVec3::new(world_offset_2d.x, 0.0, world_offset_2d.y);

        let mut density_blend: DensityField = [[[0.0; DENSITY_GRID_SIZE]; DENSITY_GRID_HEIGHT]; DENSITY_GRID_SIZE];
        let mut density_low: DensityField = [[[0.0; DENSITY_GRID_SIZE]; DENSITY_GRID_HEIGHT]; DENSITY_GRID_SIZE];
        let mut density_high: DensityField = [[[0.0; DENSITY_GRID_SIZE]; DENSITY_GRID_HEIGHT]; DENSITY_GRID_SIZE];

        // The reference doubles its height scale once, before sampling blend/low/high
        // (but after sampling the dead island fields), so all three share the doubled
        // value while the Z-ish scale (`684.412`) stays unmodified.
        self.blend_noise
            .sample_3d(&mut density_blend, world_offset_3d, DVec3::new(1368.824 / 80.0, 684.412 / 160.0, 1368.824 / 80.0));
        self.low_noise.sample_3d(&mut density_low, world_offset_3d, DVec3::splat(1368.824));
        self.high_noise.sample_3d(&mut density_high, world_offset_3d, DVec3::splat(1368.824));

        let mut density: DensityField = [[[0.0; DENSITY_GRID_SIZE]; DENSITY_GRID_HEIGHT]; DENSITY_GRID_SIZE];

        for grid_x in 0..DENSITY_GRID_SIZE {
            for grid_z in 0..DENSITY_GRID_SIZE {
                for grid_y in 0..DENSITY_GRID_HEIGHT {
                    let low = density_low[grid_x][grid_y][grid_z] / 512.0;
                    let high = density_high[grid_x][grid_y][grid_z] / 512.0;
                    let blend = (density_blend[grid_x][grid_y][grid_z] / 10.0 + 1.0) / 2.0;

                    let mut value = if blend < 0.0 {
                        low
                    } else if blend > 1.0 {
                        high
                    } else {
                        low + (high - low) * blend
                    };

                    value -= 8.0;

                    // Two independent fade zones (not an if/else!) both pushing toward
                    // `FADE_TARGET_DENSITY` - the reference applies them as two
                    // separate `if`s, so near the bottom of the small overlap between
                    // them, both apply in sequence, compounding.
                    let grid_y_i32 = grid_y as i32;

                    if grid_y_i32 > DENSITY_GRID_HEIGHT as i32 - TOP_FADE_WIDTH {
                        let fade = ((grid_y_i32 - (DENSITY_GRID_HEIGHT as i32 - TOP_FADE_WIDTH)) as f32 / (TOP_FADE_WIDTH as f32 - 1.0)) as f64;
                        value = value * (1.0 - fade) + FADE_TARGET_DENSITY * fade;
                    }
                    if grid_y_i32 < BOTTOM_FADE_WIDTH {
                        let fade = ((BOTTOM_FADE_WIDTH - grid_y_i32) as f32 / (BOTTOM_FADE_WIDTH as f32 - 1.0)) as f64;
                        value = value * (1.0 - fade) + FADE_TARGET_DENSITY * fade;
                    }

                    density[grid_x][grid_y][grid_z] = value;
                }
            }
        }

        density
    }

    fn sample_surface_fields(&self, x: i32, z: i32) -> SurfaceField {
        const SURFACE_SCALE: f64 = 1.0 / 32.0;

        let world_offset = DVec2::new((x * CHUNK_WIDTH as i32) as f64, (z * CHUNK_WIDTH as i32) as f64);

        let mut thickness_field: SurfaceField = [[0.0; CHUNK_WIDTH]; CHUNK_WIDTH];
        self.thickness_noise.sample_3d_slice(&mut thickness_field, world_offset, SURFACE_SCALE * 2.0);

        thickness_field
    }

    fn generate_surface(&self, x: i32, z: i32, column: &mut ChunkBuffer, biome_grid: &BiomeGrid, rand: &mut JavaRand, block_ids: &BlockIds) {
        let thickness_field = self.sample_surface_fields(x, z);

        // Iteration order (z outer, x inner) must match the reference generator
        // exactly - confirmed against its own `(x*16+z)` block-array indexing and
        // matching the overworld/nether generators' equivalent pass.
        for lz in 0..CHUNK_WIDTH {
            for lx in 0..CHUNK_WIDTH {
                let biome = biome_grid[lx][lz];
                let surface_thickness = (thickness_field[lx][lz] / 3.0 + 3.0 + rand.random::<f64>() * 0.25) as i32;

                carve_column(column, lx, lz, biome, surface_thickness, rand, block_ids);
            }
        }
    }
}

// Decoration reads terrain through this - always a fresh, uncached computation, same as any
// other out-of-quad read during population.
impl TerrainSource for SkyGenerator {
    fn raw_terrain(&self, x: i32, z: i32) -> ChunkBuffer {
        self.build_terrain_column(x, z)
    }

    fn carve_caves(&self, x: i32, z: i32, column: &mut ChunkBuffer) {
        CaveCarver::new(CAVE_RADIUS).carve(self.seed, x, z, column, &self.block_ids);
    }

    fn min_sub_chunk_y(&self) -> i8 {
        -4
    }

    fn dimension_sub_chunk_count(&self) -> usize {
        24
    }

    fn air_id(&self) -> i32 {
        self.block_ids.air
    }

    fn biome(&self) -> i32 {
        1
    }
}

impl ClimateSource for SkyGenerator {
    fn climate_at(&self, x: i32, z: i32) -> (f64, f64, Biome) {
        self.climate_at(x, z)
    }

    fn feature_noise_at(&self, x: f64, z: f64) -> f64 {
        self.feature_noise_at(x, z)
    }
}

fn classify_climate(temperature: f64, humidity: f64, biome_noise: f64) -> (f64, f64, Biome) {
    let climate_bias = biome_noise * 1.1 + 0.5;

    let temperature = (temperature * 0.15 + 0.7) * 0.99 + climate_bias * 0.01;
    let temperature = (1.0 - (1.0 - temperature).powi(2)).clamp(0.0, 1.0);
    let humidity = ((humidity * 0.15 + 0.5) * 0.998 + climate_bias * 0.002).clamp(0.0, 1.0);

    let biome = biome_from_climate((temperature * 63.0) as usize, (humidity * 63.0) as usize);
    (temperature, humidity, biome)
}

fn place_terrain(column: &mut ChunkBuffer, density: &DensityField, block_ids: &BlockIds) {
    for grid_x in 0..INTERP_GRID_SIZE {
        for grid_z in 0..INTERP_GRID_SIZE {
            for grid_y in 0..INTERP_GRID_HEIGHT {
                let mut x0z0 = density[grid_x][grid_y][grid_z];
                let mut x0z1 = density[grid_x][grid_y][grid_z + 1];
                let mut x1z0 = density[grid_x + 1][grid_y][grid_z];
                let mut x1z1 = density[grid_x + 1][grid_y][grid_z + 1];

                let x0z0_step = (density[grid_x][grid_y + 1][grid_z] - x0z0) / INTERP_STRIDE_Y as f64;
                let x0z1_step = (density[grid_x][grid_y + 1][grid_z + 1] - x0z1) / INTERP_STRIDE_Y as f64;
                let x1z0_step = (density[grid_x + 1][grid_y + 1][grid_z] - x1z0) / INTERP_STRIDE_Y as f64;
                let x1z1_step = (density[grid_x + 1][grid_y + 1][grid_z + 1] - x1z1) / INTERP_STRIDE_Y as f64;

                for y_step in 0..INTERP_STRIDE_Y {
                    let y = grid_y * INTERP_STRIDE_Y + y_step;

                    let edge_z0_step = (x1z0 - x0z0) / INTERP_STRIDE_XZ as f64;
                    let edge_z1_step = (x1z1 - x0z1) / INTERP_STRIDE_XZ as f64;
                    let mut edge_z0 = x0z0;
                    let mut edge_z1 = x0z1;

                    for x_step in 0..INTERP_STRIDE_XZ {
                        let x = grid_x * INTERP_STRIDE_XZ + x_step;

                        let column_step = (edge_z1 - edge_z0) / INTERP_STRIDE_XZ as f64;
                        let mut value = edge_z0;

                        for z_step in 0..INTERP_STRIDE_XZ {
                            let z = grid_z * INTERP_STRIDE_XZ + z_step;

                            let block_id = if value > 0.0 { block_ids.stone } else { block_ids.air };
                            column.set(x, y, z, block_id);

                            value += column_step;
                        }

                        edge_z0 += edge_z0_step;
                        edge_z1 += edge_z1_step;
                    }

                    x0z0 += x0z0_step;
                    x0z1 += x0z1_step;
                    x1z0 += x1z0_step;
                    x1z1 += x1z1_step;
                }
            }
        }
    }
}

fn carve_column(column: &mut ChunkBuffer, lx: usize, lz: usize, biome: Biome, surface_thickness: i32, rand: &mut JavaRand, block_ids: &BlockIds) {
    let (biome_top_id, biome_filler_id) = match biome {
        Biome::Desert | Biome::IceDesert => (block_ids.sand, block_ids.sand),
        _ => (block_ids.grass, block_ids.dirt),
    };

    // Reassigned before first use every time (the very first stone block hit always
    // takes the `remaining_thickness == -1` branch below), but declared with an
    // initial value anyway to match the reference's own local-variable shape.
    #[allow(unused_assignments)]
    let mut top_id = biome_top_id;
    let mut filler_id = biome_filler_id;
    let mut remaining_thickness: i32 = -1;

    for y in (0..CHUNK_HEIGHT as i32).rev() {
        let prev_id = column.get(lx, y as usize, lz);

        if prev_id == block_ids.air {
            remaining_thickness = -1;
        } else if prev_id == block_ids.stone {
            if remaining_thickness == -1 {
                if surface_thickness <= 0 {
                    top_id = block_ids.air;
                    filler_id = block_ids.stone;
                } else {
                    top_id = biome_top_id;
                    filler_id = biome_filler_id;
                }

                remaining_thickness = surface_thickness;
                column.set(lx, y as usize, lz, top_id);
            } else if remaining_thickness > 0 {
                column.set(lx, y as usize, lz, filler_id);

                remaining_thickness -= 1;
                if remaining_thickness == 0 && filler_id == block_ids.sand {
                    remaining_thickness = rand.random_with::<i32>(Bound::new(4));
                    filler_id = block_ids.sandstone;
                }
            }
        }
    }
}
