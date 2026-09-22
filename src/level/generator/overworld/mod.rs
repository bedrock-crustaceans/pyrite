pub mod biome;

use chorus::block::block_id;
use chorus::level::chunk::Chunk;
use chorus::level::generator::WorldGenerator;
use chorus::registry::block_registry::BlockRegistry;
use glam::{DVec2, DVec3};

use crate::level::generator::java_rand::JavaRand;
use crate::level::generator::noise::octave::OctaveNoise;
use crate::level::generator::overworld::biome::{Biome, biome_from_climate};

const CHUNK_WIDTH: usize = 16;
const CHUNK_HEIGHT: usize = 128;
const SEA_LEVEL: i32 = 64;

// Cast through f32 to match the reference generator's precision exactly.
const CLIMATE_SCALE: f64 = 0.025f32 as f64;
const CLIMATE_FREQUENCY_FACTOR: f64 = 0.25;
const HUMIDITY_SCALE: f64 = 0.05f32 as f64;
const HUMIDITY_FREQUENCY_FACTOR: f64 = 1.0 / 3.0;
const BIOME_SCALE: f64 = 0.25;
const BIOME_FREQUENCY_FACTOR: f64 = 0.5882352941176471;

const DENSITY_GRID_SIZE: usize = 5;
const DENSITY_GRID_HEIGHT: usize = 17;
const DENSITY_SAMPLE_STRIDE: usize = CHUNK_WIDTH / DENSITY_GRID_SIZE;

const INTERP_GRID_SIZE: usize = DENSITY_GRID_SIZE - 1;
const INTERP_GRID_HEIGHT: usize = DENSITY_GRID_HEIGHT - 1;
const INTERP_STRIDE_XZ: usize = CHUNK_WIDTH / INTERP_GRID_SIZE;
const INTERP_STRIDE_Y: usize = CHUNK_HEIGHT / INTERP_GRID_HEIGHT;

type ClimateField = [[f64; CHUNK_WIDTH]; CHUNK_WIDTH];
type SurfaceField = [[f64; CHUNK_WIDTH]; CHUNK_WIDTH];
type BiomeGrid = [[Biome; CHUNK_WIDTH]; CHUNK_WIDTH];
type ReliefField = [[f64; DENSITY_GRID_SIZE]; DENSITY_GRID_SIZE];
type DensityField = [[[f64; DENSITY_GRID_SIZE]; DENSITY_GRID_HEIGHT]; DENSITY_GRID_SIZE];

struct BlockIds {
    air: i32,
    stone: i32,
    water: i32,
    ice: i32,
    bedrock: i32,
    grass: i32,
    dirt: i32,
    sand: i32,
    sandstone: i32,
    gravel: i32,
}

impl BlockIds {
    fn resolve(registry: &BlockRegistry) -> Self {
        let id = |identifier: &str| registry.get_block_id(identifier).unwrap_or(0);
        Self {
            air: id(block_id::AIR),
            stone: id(block_id::STONE),
            water: id(block_id::WATER),
            ice: id(block_id::ICE),
            bedrock: id(block_id::BEDROCK),
            grass: id(block_id::GRASS_BLOCK),
            dirt: id(block_id::DIRT),
            sand: id(block_id::SAND),
            sandstone: id(block_id::SANDSTONE),
            gravel: id(block_id::GRAVEL),
        }
    }
}

pub struct OverworldGenerator {
    pub seed: i64,

    temperature_noise: OctaveNoise,
    humidity_noise: OctaveNoise,
    biome_noise: OctaveNoise,

    terrain_noise_0: OctaveNoise,
    terrain_noise_1: OctaveNoise,
    terrain_noise_2: OctaveNoise,
    terrain_noise_3: OctaveNoise,
    terrain_noise_4: OctaveNoise,

    sand_gravel_noise: OctaveNoise,
    thickness_noise: OctaveNoise,
}

impl OverworldGenerator {
    pub fn new(seed: i64) -> Self {
        let mut rand = JavaRand::new(seed);

        Self {
            seed,

            temperature_noise: OctaveNoise::new(&mut JavaRand::new(seed.wrapping_mul(9871)), 4),
            humidity_noise: OctaveNoise::new(&mut JavaRand::new(seed.wrapping_mul(39811)), 4),
            biome_noise: OctaveNoise::new(&mut JavaRand::new(seed.wrapping_mul(543321)), 2),

            terrain_noise_0: OctaveNoise::new(&mut rand, 16),
            terrain_noise_1: OctaveNoise::new(&mut rand, 16),
            terrain_noise_2: OctaveNoise::new(&mut rand, 8),

            sand_gravel_noise: OctaveNoise::new(&mut rand, 4),
            thickness_noise: OctaveNoise::new(&mut rand, 4),

            terrain_noise_3: OctaveNoise::new(&mut rand, 10),
            terrain_noise_4: OctaveNoise::new(&mut rand, 16),
        }
    }

    fn generate_biomes(&self, x: i32, z: i32, temperature_grid: &mut ClimateField, humidity_grid: &mut ClimateField) -> BiomeGrid {
        let world_offset = DVec2::new((x * CHUNK_WIDTH as i32) as f64, (z * CHUNK_WIDTH as i32) as f64);

        let mut biome_noise_grid: ClimateField = [[0.0; CHUNK_WIDTH]; CHUNK_WIDTH];

        self.temperature_noise.sample_simplex_2d(temperature_grid, world_offset, DVec2::splat(CLIMATE_SCALE), CLIMATE_FREQUENCY_FACTOR);
        self.humidity_noise.sample_simplex_2d(humidity_grid, world_offset, DVec2::splat(HUMIDITY_SCALE), HUMIDITY_FREQUENCY_FACTOR);
        self.biome_noise.sample_simplex_2d(&mut biome_noise_grid, world_offset, DVec2::splat(BIOME_SCALE), BIOME_FREQUENCY_FACTOR);

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

    /// Builds the 5x17x5 terrain density field: positive values become stone, everything
    /// else stays open (water gets filled in separately once this is placed into blocks).
    fn build_density_field(&self, x: i32, z: i32, temperature_grid: &ClimateField, humidity_grid: &ClimateField) -> DensityField {
        let world_offset_2d = DVec2::new((x * INTERP_GRID_SIZE as i32) as f64, (z * INTERP_GRID_SIZE as i32) as f64);
        let world_offset_3d = DVec3::new(world_offset_2d.x, 0.0, world_offset_2d.y);

        let mut low_relief: ReliefField = [[0.0; DENSITY_GRID_SIZE]; DENSITY_GRID_SIZE];
        let mut high_relief: ReliefField = [[0.0; DENSITY_GRID_SIZE]; DENSITY_GRID_SIZE];
        let mut density_blend: DensityField = [[[0.0; DENSITY_GRID_SIZE]; DENSITY_GRID_HEIGHT]; DENSITY_GRID_SIZE];
        let mut density_low: DensityField = [[[0.0; DENSITY_GRID_SIZE]; DENSITY_GRID_HEIGHT]; DENSITY_GRID_SIZE];
        let mut density_high: DensityField = [[[0.0; DENSITY_GRID_SIZE]; DENSITY_GRID_HEIGHT]; DENSITY_GRID_SIZE];

        self.terrain_noise_3.sample_2d(&mut low_relief, world_offset_2d, DVec2::splat(1.121));
        self.terrain_noise_4.sample_2d(&mut high_relief, world_offset_2d, DVec2::splat(200.0));
        self.terrain_noise_2.sample_3d(&mut density_blend, world_offset_3d, DVec3::new(684.412 / 80.0, 684.412 / 160.0, 684.412 / 80.0));
        self.terrain_noise_0.sample_3d(&mut density_low, world_offset_3d, DVec3::splat(684.412));
        self.terrain_noise_1.sample_3d(&mut density_high, world_offset_3d, DVec3::splat(684.412));

        let mut density: DensityField = [[[0.0; DENSITY_GRID_SIZE]; DENSITY_GRID_HEIGHT]; DENSITY_GRID_SIZE];

        for grid_x in 0..DENSITY_GRID_SIZE {
            let sample_x = grid_x * DENSITY_SAMPLE_STRIDE + DENSITY_SAMPLE_STRIDE / 2;
            for grid_z in 0..DENSITY_GRID_SIZE {
                let sample_z = grid_z * DENSITY_SAMPLE_STRIDE + DENSITY_SAMPLE_STRIDE / 2;

                let temperature = temperature_grid[sample_x][sample_z];
                let humidity = humidity_grid[sample_x][sample_z] * temperature;
                let humidity_falloff = 1.0 - (1.0 - humidity).powi(4);

                let mut relief = ((low_relief[grid_x][grid_z] + 256.0) / 512.0 * humidity_falloff).min(1.0);

                let mut height_bias = high_relief[grid_x][grid_z] / 8000.0;
                if height_bias < 0.0 {
                    height_bias = -height_bias * 0.3;
                }
                height_bias = height_bias * 3.0 - 2.0;

                if height_bias < 0.0 {
                    height_bias /= 2.0;
                    height_bias = height_bias.max(-1.0);
                    height_bias /= 1.4;
                    height_bias /= 2.0;
                    relief = 0.0;
                } else {
                    height_bias = height_bias.min(1.0);
                    height_bias /= 8.0;
                }

                relief = relief.max(0.0) + 0.5;

                height_bias = height_bias * DENSITY_GRID_HEIGHT as f64 / 16.0;
                let base_height = DENSITY_GRID_HEIGHT as f64 / 2.0 + height_bias * 4.0;

                for grid_y in 0..DENSITY_GRID_HEIGHT {
                    let mut height_falloff = (grid_y as f64 - base_height) * 12.0 / relief;
                    if height_falloff < 0.0 {
                        height_falloff *= 4.0;
                    }

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

                    value -= height_falloff;

                    if grid_y > DENSITY_GRID_HEIGHT - 4 {
                        let fade = ((grid_y - (DENSITY_GRID_HEIGHT - 4)) as f32 / 3.0) as f64;
                        value = value * (1.0 - fade) + (-10.0 * fade);
                    }

                    density[grid_x][grid_y][grid_z] = value;
                }
            }
        }

        density
    }

    fn generate_terrain(&self, x: i32, z: i32, chunk: &mut Chunk, temperature_grid: &ClimateField, humidity_grid: &ClimateField, block_ids: &BlockIds) {
        let density = self.build_density_field(x, z, temperature_grid, humidity_grid);
        place_terrain(chunk, &density, temperature_grid, block_ids);
    }

    fn generate_surface(&self, x: i32, z: i32, chunk: &mut Chunk, biome_grid: &BiomeGrid, rand: &mut JavaRand, block_ids: &BlockIds) {
        const SURFACE_SCALE: f64 = 1.0 / 32.0;

        let world_offset = DVec2::new((x * CHUNK_WIDTH as i32) as f64, (z * CHUNK_WIDTH as i32) as f64);

        // sample_3d_slice and sample_2d below sample different noise paths and are not
        // interchangeable - see their docs. The reference generator's sand and thickness
        // fields use the former, its gravel field the latter.
        let mut sand_field: SurfaceField = [[0.0; CHUNK_WIDTH]; CHUNK_WIDTH];
        let mut gravel_field: SurfaceField = [[0.0; CHUNK_WIDTH]; CHUNK_WIDTH];
        let mut thickness_field: SurfaceField = [[0.0; CHUNK_WIDTH]; CHUNK_WIDTH];

        self.sand_gravel_noise.sample_3d_slice(&mut sand_field, world_offset, SURFACE_SCALE);
        self.sand_gravel_noise.sample_2d(&mut gravel_field, world_offset, DVec2::splat(SURFACE_SCALE));
        self.thickness_noise.sample_3d_slice(&mut thickness_field, world_offset, SURFACE_SCALE * 2.0);

        // Iteration order (z outer, x inner) must match the reference generator exactly:
        // it determines how the per-column random calls below line up with world position.
        for lz in 0..CHUNK_WIDTH {
            for lx in 0..CHUNK_WIDTH {
                let biome = biome_grid[lx][lz];
                let has_sand = sand_field[lx][lz] + rand.next_double() * 0.2 > 0.0;
                let has_gravel = gravel_field[lx][lz] + rand.next_double() * 0.2 > 3.0;
                let surface_thickness = (thickness_field[lx][lz] / 3.0 + 3.0 + rand.next_double() * 0.25) as i32;

                carve_column(chunk, lx as u8, lz as u8, biome, has_sand, has_gravel, surface_thickness, rand, block_ids);
            }
        }
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

/// Trilinearly interpolates the density field up to full block resolution and places
/// stone (density > 0) or water/ice (below sea level) accordingly.
fn place_terrain(chunk: &mut Chunk, density: &DensityField, temperature_grid: &ClimateField, block_ids: &BlockIds) {
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
                            let temperature = temperature_grid[x][z];

                            let mut block_id = None;
                            if y < SEA_LEVEL as usize {
                                block_id = Some(if temperature < 0.5 && y == 63 { block_ids.ice } else { block_ids.water });
                            }
                            if value > 0.0 {
                                block_id = Some(block_ids.stone);
                            }

                            if let Some(block_id) = block_id {
                                chunk.set_block(x as u8, y as i32, z as u8, 0, block_id);
                            }

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

/// Carves one column down to bedrock: biome-appropriate grass/dirt or sand on top,
/// sand/gravel patches and sandstone under thick sand near sea level, stone below.
#[allow(clippy::too_many_arguments)]
fn carve_column(chunk: &mut Chunk, lx: u8, lz: u8, biome: Biome, has_sand: bool, has_gravel: bool, surface_thickness: i32, rand: &mut JavaRand, block_ids: &BlockIds) {
    let (biome_top_id, biome_filler_id) = match biome {
        Biome::Desert | Biome::IceDesert => (block_ids.sand, block_ids.sand),
        _ => (block_ids.grass, block_ids.dirt),
    };

    let mut top_id = biome_top_id;
    let mut filler_id = biome_filler_id;
    let mut remaining_thickness: i32 = -1;

    for y in (0..CHUNK_HEIGHT as i32).rev() {
        if y <= rand.next_i32_bounded(5) {
            chunk.set_block(lx, y, lz, 0, block_ids.bedrock);
            continue;
        }

        let prev_id = chunk.get_block(lx, y, lz, 0).unwrap_or(block_ids.air);

        if prev_id == block_ids.air {
            remaining_thickness = -1;
        } else if prev_id == block_ids.stone {
            if remaining_thickness == -1 {
                if surface_thickness <= 0 {
                    top_id = block_ids.air;
                    filler_id = block_ids.stone;
                } else if (SEA_LEVEL - 4..=SEA_LEVEL + 1).contains(&y) {
                    top_id = biome_top_id;
                    filler_id = biome_filler_id;

                    if has_sand {
                        top_id = block_ids.sand;
                        filler_id = block_ids.sand;
                    } else if has_gravel {
                        top_id = block_ids.air;
                        filler_id = block_ids.gravel;
                    }
                }

                if y < SEA_LEVEL && top_id == block_ids.air {
                    top_id = block_ids.water;
                }

                remaining_thickness = surface_thickness;

                if y >= SEA_LEVEL - 1 {
                    chunk.set_block(lx, y, lz, 0, top_id);
                } else {
                    chunk.set_block(lx, y, lz, 0, filler_id);
                }
            } else if remaining_thickness > 0 {
                chunk.set_block(lx, y, lz, 0, filler_id);

                remaining_thickness -= 1;
                if remaining_thickness == 0 && filler_id == block_ids.sand {
                    remaining_thickness = rand.next_i32_bounded(4);
                    filler_id = block_ids.sandstone;
                }
            }
        }
    }
}

impl WorldGenerator for OverworldGenerator {
    fn generate(&self, registry: &BlockRegistry, x: i32, z: i32, chunk: &mut Chunk) {
        let block_ids = BlockIds::resolve(registry);

        let mut temperature_grid: ClimateField = [[0.0; CHUNK_WIDTH]; CHUNK_WIDTH];
        let mut humidity_grid: ClimateField = [[0.0; CHUNK_WIDTH]; CHUNK_WIDTH];
        let biome_grid = self.generate_biomes(x, z, &mut temperature_grid, &mut humidity_grid);

        self.generate_terrain(x, z, chunk, &temperature_grid, &humidity_grid, &block_ids);

        let chunk_seed = (x as i64).wrapping_mul(341873128712).wrapping_add((z as i64).wrapping_mul(132897987541));
        let mut rand = JavaRand::new(chunk_seed);

        self.generate_surface(x, z, chunk, &biome_grid, &mut rand, &block_ids);
    }
}
