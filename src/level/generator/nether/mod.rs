mod cave;
mod phases;
mod population;

use chorus::block::block_id;
use chorus::registry::block_registry::BlockRegistry;
use glam::{DVec2, DVec3};

use crate::level::generator::noise::octave::OctaveNoise;
use crate::level::generator::shared::chunk_buffer::ChunkBuffer;
use crate::level::generator::shared::{CAVE_RADIUS, CHUNK_HEIGHT, CHUNK_WIDTH, TerrainSource, chunk_seed};
use crate::rand::java::JavaRand;

use crate::rand::primitives::Bound;
use cave::CaveCarver;

const DENSITY_GRID_SIZE: usize = 5;
const DENSITY_GRID_HEIGHT: usize = 17;

const INTERP_GRID_SIZE: usize = DENSITY_GRID_SIZE - 1;
const INTERP_GRID_HEIGHT: usize = DENSITY_GRID_HEIGHT - 1;
const INTERP_STRIDE_XZ: usize = CHUNK_WIDTH / INTERP_GRID_SIZE;
const INTERP_STRIDE_Y: usize = CHUNK_HEIGHT / INTERP_GRID_HEIGHT;

const LAVA_LEVEL: i32 = 32;
const SURFACE_BAND_CENTER: i32 = 64;

type DensityField = [[[f64; DENSITY_GRID_SIZE]; DENSITY_GRID_HEIGHT]; DENSITY_GRID_SIZE];
type SurfaceField = [[f64; CHUNK_WIDTH]; CHUNK_WIDTH];

#[derive(Debug)]
pub struct BlockIds {
    air: i32,
    netherrack: i32,
    lava: i32,
    lava_still: i32,
    bedrock: i32,
    soul_sand: i32,
    gravel: i32,
    glowstone: i32,
    fire: i32,
    brown_mushroom: i32,
    red_mushroom: i32,
}

impl BlockIds {
    fn resolve(registry: &BlockRegistry) -> Self {
        let id = |identifier: &str| registry.get_block_id(identifier).unwrap_or(0);
        Self {
            air: id(block_id::AIR),
            netherrack: id(block_id::NETHERRACK),
            lava: id(block_id::FLOWING_LAVA),
            lava_still: id(block_id::LAVA),
            bedrock: id(block_id::BEDROCK),
            soul_sand: id(block_id::SOUL_SAND),
            gravel: id(block_id::GRAVEL),
            glowstone: id(block_id::GLOWSTONE),
            fire: id(block_id::FIRE),
            brown_mushroom: id(block_id::BROWN_MUSHROOM),
            red_mushroom: id(block_id::RED_MUSHROOM),
        }
    }
}

pub struct NetherGenerator {
    pub seed: i64,
    block_ids: BlockIds,

    low_noise: OctaveNoise,
    high_noise: OctaveNoise,
    blend_noise: OctaveNoise,
    sand_gravel_noise: OctaveNoise,
    thickness_noise: OctaveNoise,
}

impl NetherGenerator {
    pub fn new(seed: i64, registry: &BlockRegistry) -> Self {
        let mut rand = JavaRand::new(seed);

        Self {
            seed,
            block_ids: BlockIds::resolve(registry),

            // Construction order matters (each draws from the same `rand`) - matches
            // `ChunkProviderHell`'s own field order exactly. Two of its 7 noise fields
            // (10 and 16 octaves, sampled last in its density builder) are omitted:
            // their sampled values feed local variables that are computed and then
            // never read anywhere before being overwritten - dead code in the
            // reference itself. Skipping their *construction* too is safe only
            // because they're last in the reference's own construction order, so
            // nothing built afterward depends on the RNG state they'd have consumed.
            low_noise: OctaveNoise::new(&mut rand, 16),
            high_noise: OctaveNoise::new(&mut rand, 16),
            blend_noise: OctaveNoise::new(&mut rand, 8),
            sand_gravel_noise: OctaveNoise::new(&mut rand, 4),
            thickness_noise: OctaveNoise::new(&mut rand, 4),
        }
    }

    fn build_terrain_column(&self, x: i32, z: i32) -> ChunkBuffer {
        let block_ids = &self.block_ids;
        let mut column = ChunkBuffer::new(block_ids.air);

        let density = self.build_density_field(x, z);
        place_terrain(&mut column, &density, block_ids);

        let mut rand = JavaRand::new(i64::wrapping_add((x as i64).wrapping_mul(341873128712), (z as i64).wrapping_mul(132897987541)));
        self.generate_surface(x, z, &mut column, &mut rand, block_ids);

        column
    }

    #[allow(clippy::needless_range_loop)]
    fn build_density_field(&self, x: i32, z: i32) -> DensityField {
        let world_offset_2d = DVec2::new((x * INTERP_GRID_SIZE as i32) as f64, (z * INTERP_GRID_SIZE as i32) as f64);
        let world_offset_3d = DVec3::new(world_offset_2d.x, 0.0, world_offset_2d.y);

        let mut density_blend: DensityField = [[[0.0; DENSITY_GRID_SIZE]; DENSITY_GRID_HEIGHT]; DENSITY_GRID_SIZE];
        let mut density_low: DensityField = [[[0.0; DENSITY_GRID_SIZE]; DENSITY_GRID_HEIGHT]; DENSITY_GRID_SIZE];
        let mut density_high: DensityField = [[[0.0; DENSITY_GRID_SIZE]; DENSITY_GRID_HEIGHT]; DENSITY_GRID_SIZE];

        self.blend_noise
            .sample_3d(&mut density_blend, world_offset_3d, DVec3::new(684.412 / 80.0, 2053.236 / 60.0, 684.412 / 80.0));
        self.low_noise.sample_3d(&mut density_low, world_offset_3d, DVec3::new(684.412, 2053.236, 684.412));
        self.high_noise.sample_3d(&mut density_high, world_offset_3d, DVec3::new(684.412, 2053.236, 684.412));

        // Cosine-based ceiling/floor taper: pulls density down near the very top and
        // bottom of the height range (with an extra cubic dip in the outer 4 cells),
        // giving the nether its closed-cavern shape instead of overworld's single
        // open surface. The reference computes this with real `Math.cos`, not the
        // Notchian sin/cos lookup table used everywhere else in this generator.
        // `y` also feeds the formula itself, not just `taper`'s indexing, so clippy's
        // iterator rewrite doesn't apply here.
        let mut taper = [0.0f64; DENSITY_GRID_HEIGHT];
        for y in 0..DENSITY_GRID_HEIGHT {
            taper[y] = (y as f64 * std::f64::consts::PI * 6.0 / DENSITY_GRID_HEIGHT as f64).cos() * 2.0;

            let mut edge_distance = y as f64;
            if y > DENSITY_GRID_HEIGHT / 2 {
                edge_distance = (DENSITY_GRID_HEIGHT - 1 - y) as f64;
            }
            if edge_distance < 4.0 {
                edge_distance = 4.0 - edge_distance;
                taper[y] -= edge_distance.powi(3) * 10.0;
            }
        }

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

                    value -= taper[grid_y];

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

    fn sample_surface_fields(&self, x: i32, z: i32) -> (SurfaceField, SurfaceField, SurfaceField) {
        const SURFACE_SCALE: f64 = 1.0 / 32.0;

        let world_offset = DVec2::new((x * CHUNK_WIDTH as i32) as f64, (z * CHUNK_WIDTH as i32) as f64);

        let mut soul_sand_field: SurfaceField = [[0.0; CHUNK_WIDTH]; CHUNK_WIDTH];
        let mut gravel_field: SurfaceField = [[0.0; CHUNK_WIDTH]; CHUNK_WIDTH];
        let mut thickness_field: SurfaceField = [[0.0; CHUNK_WIDTH]; CHUNK_WIDTH];

        self.sand_gravel_noise.sample_3d_slice(&mut soul_sand_field, world_offset, SURFACE_SCALE);
        self.sand_gravel_noise.sample_2d(&mut gravel_field, world_offset, DVec2::splat(SURFACE_SCALE));
        self.thickness_noise.sample_3d_slice(&mut thickness_field, world_offset, SURFACE_SCALE * 2.0);

        (soul_sand_field, gravel_field, thickness_field)
    }

    fn generate_surface(&self, x: i32, z: i32, column: &mut ChunkBuffer, rand: &mut JavaRand, block_ids: &BlockIds) {
        let (soul_sand_field, gravel_field, thickness_field) = self.sample_surface_fields(x, z);

        // Iteration order (z outer, x inner) must match the reference generator
        // exactly - confirmed against its own `var7 + var8 * 16` indexing, the same
        // outer/inner correspondence the overworld generator's equivalent pass uses.
        for lz in 0..CHUNK_WIDTH {
            for lx in 0..CHUNK_WIDTH {
                let has_soul_sand = soul_sand_field[lx][lz] + rand.random::<f64>() * 0.2 > 0.0;
                let has_gravel = gravel_field[lx][lz] + rand.random::<f64>() * 0.2 > 0.0;
                let surface_thickness = (thickness_field[lx][lz] / 3.0 + 3.0 + rand.random::<f64>() * 0.25) as i32;

                carve_column(column, lx, lz, has_soul_sand, has_gravel, surface_thickness, rand, block_ids);
            }
        }
    }
}

// Decoration reads terrain through this - always a fresh, uncached computation, same as any
// other out-of-quad read during population.
impl TerrainSource for NetherGenerator {
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

                            let mut block_id = if (y as i32) < LAVA_LEVEL { block_ids.lava_still } else { block_ids.air };
                            if value > 0.0 {
                                block_id = block_ids.netherrack;
                            }

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

#[allow(clippy::too_many_arguments)]
fn carve_column(column: &mut ChunkBuffer, lx: usize, lz: usize, has_soul_sand: bool, has_gravel: bool, surface_thickness: i32, rand: &mut JavaRand, block_ids: &BlockIds) {
    let mut top_id = block_ids.netherrack;
    let mut filler_id = block_ids.netherrack;
    let mut remaining_thickness: i32 = -1;

    for y in (0..CHUNK_HEIGHT as i32).rev() {
        // The reference draws a separate `nextInt(5)` for the ceiling and floor
        // bedrock bands - same resulting body, but each must consume its own draw,
        // so this can't be collapsed into one condition.
        #[allow(clippy::if_same_then_else)]
        if y >= CHUNK_HEIGHT as i32 - 1 - rand.random_with::<i32>(Bound::new(5)) {
            column.set(lx, y as usize, lz, block_ids.bedrock);
            continue;
        } else if y <= rand.random_with::<i32>(Bound::new(5)) {
            column.set(lx, y as usize, lz, block_ids.bedrock);
            continue;
        }

        let prev_id = column.get(lx, y as usize, lz);

        if prev_id == block_ids.air {
            remaining_thickness = -1;
        } else if prev_id == block_ids.netherrack {
            if remaining_thickness == -1 {
                if surface_thickness <= 0 {
                    top_id = block_ids.air;
                    filler_id = block_ids.netherrack;
                } else if (SURFACE_BAND_CENTER - 4..=SURFACE_BAND_CENTER + 1).contains(&y) {
                    top_id = block_ids.netherrack;
                    filler_id = block_ids.netherrack;

                    if has_gravel {
                        top_id = block_ids.gravel;
                    }
                    if has_gravel {
                        filler_id = block_ids.netherrack;
                    }
                    if has_soul_sand {
                        top_id = block_ids.soul_sand;
                    }
                    if has_soul_sand {
                        filler_id = block_ids.soul_sand;
                    }
                }

                if y < SURFACE_BAND_CENTER && top_id == block_ids.air {
                    top_id = block_ids.lava_still;
                }

                remaining_thickness = surface_thickness;
                if y >= SURFACE_BAND_CENTER - 1 {
                    column.set(lx, y as usize, lz, top_id);
                } else {
                    column.set(lx, y as usize, lz, filler_id);
                }
            } else if remaining_thickness > 0 {
                remaining_thickness -= 1;
                column.set(lx, y as usize, lz, filler_id);
            }
        }
    }
}
