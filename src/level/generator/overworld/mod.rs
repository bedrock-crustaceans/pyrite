pub mod biome;
mod cave;
mod column;
mod dungeon;
mod lake;
mod owner_buffer;
mod plant;
mod population;
mod spring;
mod tree;
mod vein;

use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};

use chorus::block::block_id;
use chorus::level::chunk::Chunk;
use chorus::level::generator::WorldGenerator;
use chorus::registry::block_registry::BlockRegistry;
use glam::{DVec2, DVec3};

use crate::level::generator::java_rand::JavaRand;
use crate::level::generator::noise::octave::OctaveNoise;
use crate::level::generator::overworld::biome::{Biome, biome_from_climate};
use crate::level::generator::overworld::cave::CaveCarver;
use crate::level::generator::overworld::column::Column;

const CHUNK_WIDTH: usize = 16;
const CHUNK_HEIGHT: usize = 128;
const SEA_LEVEL: i32 = 64;

const SUB_CHUNK_SIZE: usize = 16;
const SUB_CHUNK_COUNT: usize = CHUNK_HEIGHT / SUB_CHUNK_SIZE;
const CAVE_RADIUS: i32 = 8;

/// How many distinct chunks' terrain-and-caves columns `TerrainCaveCache` keeps warm
/// at once. Population only ever needs the 4 owners `{x-1, x} x {z-1, z}` per chunk
/// (see `population::populate`'s doc comment), but chunks are typically generated in a
/// scrolling, locality-preserving order, so a modest capacity keeps the cave-carve
/// cost of each distinct neighbor amortized to roughly once, rather than repeated for
/// every chunk that happens to touch it.
const TERRAIN_CAVE_CACHE_CAPACITY: usize = 256;

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

#[derive(Debug)]
pub(crate) struct BlockIds {
    air: i32,
    stone: i32,
    water: i32,
    water_flowing: i32,
    ice: i32,
    bedrock: i32,
    grass: i32,
    dirt: i32,
    sand: i32,
    sandstone: i32,
    gravel: i32,
    lava: i32,
    lava_still: i32,
    clay: i32,
    cobblestone: i32,
    mossy_cobblestone: i32,
    chest: i32,
    mob_spawner: i32,
    coal_ore: i32,
    iron_ore: i32,
    gold_ore: i32,
    redstone_ore: i32,
    diamond_ore: i32,
    lapis_ore: i32,
    oak_log: i32,
    oak_leaves: i32,
    birch_log: i32,
    birch_leaves: i32,
    spruce_log: i32,
    spruce_leaves: i32,
    dandelion: i32,
    poppy: i32,
    tall_grass: i32,
    fern: i32,
    deadbush: i32,
    red_mushroom: i32,
    brown_mushroom: i32,
    reeds: i32,
    pumpkin: i32,
    cactus: i32,
}

impl BlockIds {
    fn resolve(registry: &BlockRegistry) -> Self {
        let id = |identifier: &str| registry.get_block_id(identifier).unwrap_or(0);
        Self {
            air: id(block_id::AIR),
            stone: id(block_id::STONE),
            water: id(block_id::WATER),
            water_flowing: id(block_id::FLOWING_WATER),
            ice: id(block_id::ICE),
            bedrock: id(block_id::BEDROCK),
            grass: id(block_id::GRASS_BLOCK),
            dirt: id(block_id::DIRT),
            sand: id(block_id::SAND),
            sandstone: id(block_id::SANDSTONE),
            gravel: id(block_id::GRAVEL),
            lava: id(block_id::FLOWING_LAVA),
            lava_still: id(block_id::LAVA),
            clay: id(block_id::CLAY),
            cobblestone: id(block_id::COBBLESTONE),
            mossy_cobblestone: id(block_id::MOSSY_COBBLESTONE),
            chest: id(block_id::CHEST),
            mob_spawner: id(block_id::MOB_SPAWNER),
            coal_ore: id(block_id::COAL_ORE),
            iron_ore: id(block_id::IRON_ORE),
            gold_ore: id(block_id::GOLD_ORE),
            redstone_ore: id(block_id::REDSTONE_ORE),
            diamond_ore: id(block_id::DIAMOND_ORE),
            lapis_ore: id(block_id::LAPIS_ORE),
            oak_log: id(block_id::OAK_LOG),
            oak_leaves: id(block_id::OAK_LEAVES),
            birch_log: id(block_id::BIRCH_LOG),
            birch_leaves: id(block_id::BIRCH_LEAVES),
            spruce_log: id(block_id::SPRUCE_LOG),
            spruce_leaves: id(block_id::SPRUCE_LEAVES),
            dandelion: id(block_id::DANDELION),
            poppy: id(block_id::POPPY),
            tall_grass: id(block_id::TALL_GRASS),
            fern: id(block_id::FERN),
            deadbush: id(block_id::DEADBUSH),
            red_mushroom: id(block_id::RED_MUSHROOM),
            brown_mushroom: id(block_id::BROWN_MUSHROOM),
            reeds: id(block_id::REEDS),
            pumpkin: id(block_id::CARVED_PUMPKIN),
            cactus: id(block_id::CACTUS),
        }
    }
}

/// Derives the deterministic per-chunk seed used to reproduce another chunk's cave
/// carving or population features from scratch, without needing that chunk to exist.
pub(super) fn owner_chunk_seed(world_seed: i64, x: i32, z: i32) -> i64 {
    let mut rand = JavaRand::new(world_seed);
    let x_mul = rand.next_i64().wrapping_div(2).wrapping_mul(2).wrapping_add(1);
    let z_mul = rand.next_i64().wrapping_div(2).wrapping_mul(2).wrapping_add(1);
    (x as i64).wrapping_mul(x_mul).wrapping_add((z as i64).wrapping_mul(z_mul)) ^ world_seed
}

/// One 16-block cubic slice of a generated column, indexed locally as `[x][y][z]`.
pub(crate) struct SubChunkBlocks {
    blocks: [[[i32; SUB_CHUNK_SIZE]; SUB_CHUNK_SIZE]; SUB_CHUNK_SIZE],
}

/// FIFO-evicted cache keyed by `(seed, x, z)`, outliving a single chunk's own
/// `generate()` call unlike `column_cache` - population needs the same neighboring
/// chunk's data repeatedly as different chunks around it get generated in turn.
struct BoundedCache<V> {
    capacity: usize,
    order: VecDeque<(i64, i32, i32)>,
    entries: HashMap<(i64, i32, i32), Arc<V>>,
}

impl<V> BoundedCache<V> {
    fn new(capacity: usize) -> Self {
        Self {
            capacity,
            order: VecDeque::new(),
            entries: HashMap::new(),
        }
    }

    fn get(&self, key: &(i64, i32, i32)) -> Option<Arc<V>> {
        self.entries.get(key).cloned()
    }

    fn insert(&mut self, key: (i64, i32, i32), value: Arc<V>) {
        if self.entries.insert(key, value).is_none() {
            self.order.push_back(key);
            if self.order.len() > self.capacity
                && let Some(oldest) = self.order.pop_front()
            {
                self.entries.remove(&oldest);
            }
        }
    }
}

pub struct OverworldGenerator {
    pub seed: i64,
    block_ids: BlockIds,

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
    feature_noise: OctaveNoise,

    /// Caches each chunk's column across its 8 sequential `generate_sub_chunk` calls,
    /// keyed by seed as well as position in case more than one generator instance is
    /// ever in play at once.
    column_cache: Mutex<HashMap<(i64, i32, i32), Arc<Column>>>,

    /// Each chunk's terrain-and-caves column, with no population applied.
    terrain_cave_cache: Mutex<BoundedCache<Column>>,

    /// Each owner chunk's fully-populated `OwnerBuffer` (lakes through springs).
    owner_population_cache: Mutex<BoundedCache<owner_buffer::OwnerBuffer>>,

    /// Same as `owner_population_cache`, but seeded from virgin terrain even in its
    /// own home chunk - see `owner_population_isolated`.
    owner_population_isolated_cache: Mutex<BoundedCache<owner_buffer::OwnerBuffer>>,
}

impl OverworldGenerator {
    pub fn new(seed: i64, registry: &BlockRegistry) -> Self {
        let mut rand = JavaRand::new(seed);

        Self {
            seed,
            block_ids: BlockIds::resolve(registry),

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
            feature_noise: OctaveNoise::new(&mut rand, 8),

            column_cache: Mutex::new(HashMap::new()),
            terrain_cave_cache: Mutex::new(BoundedCache::new(TERRAIN_CAVE_CACHE_CAPACITY)),
            owner_population_cache: Mutex::new(BoundedCache::new(TERRAIN_CAVE_CACHE_CAPACITY)),
            owner_population_isolated_cache: Mutex::new(BoundedCache::new(TERRAIN_CAVE_CACHE_CAPACITY)),
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

    /// Generates terrain and surface only (no caves, no population features) for a
    /// whole chunk column - the first half of `build_column`. For a single column,
    /// prefer the much cheaper `terrain_column_at`.
    fn build_terrain_column(&self, x: i32, z: i32) -> Column {
        let block_ids = &self.block_ids;
        let mut column = Column::new(block_ids.air);

        let mut temperature_grid: ClimateField = [[0.0; CHUNK_WIDTH]; CHUNK_WIDTH];
        let mut humidity_grid: ClimateField = [[0.0; CHUNK_WIDTH]; CHUNK_WIDTH];

        let biome_grid = self.generate_biomes(x, z, &mut temperature_grid, &mut humidity_grid);

        self.generate_terrain(x, z, &mut column, &temperature_grid, &humidity_grid, block_ids);

        let mut rand = JavaRand::new(i64::wrapping_add((x as i64).wrapping_mul(341873128712), (z as i64).wrapping_mul(132897987541)));

        self.generate_surface(x, z, &mut column, &biome_grid, &mut rand, block_ids);

        column
    }

    /// Samples classified (temperature, humidity, biome) at a single absolute
    /// position, independent of any chunk's 16x16 climate grid - matches the
    /// reference's own single-point `get_biome`, generalized to return the classified
    /// temperature/humidity too since density sampling needs those, not just the biome.
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

    /// Samples the raw feature-density noise at a single absolute (x, z) position,
    /// used to bias how many trees a chunk gets.
    fn feature_noise_at(&self, x: f64, z: f64) -> f64 {
        let mut value = [[[0.0f64; 1]; 1]; 1];
        self.feature_noise.sample_3d(&mut value, DVec3::new(x, z, 0.0), DVec3::ONE);
        value[0][0][0]
    }

    /// Generates one full chunk column: biomes, terrain, surface, caves and population
    /// features, in that order, since each stage reads blocks placed by the previous
    /// one. Caves alone scan an 8-chunk radius, so the result is cached (see `column`)
    /// for the rest of this chunk's subchunks rather than recomputed per subchunk.
    fn build_column(&self, x: i32, z: i32) -> Column {
        let mut column = (*self.terrain_and_caves(x, z)).clone();
        population::populate(self, x, z, &mut column);
        column
    }

    /// Chunk `(x, z)`'s terrain and caves with no population applied - the base every
    /// `OwnerBuffer` clones from. Has to include caves, not just terrain: a cave can
    /// remove the ground under a spot terrain alone says is solid, which would
    /// otherwise let a tree/plant grow somewhere a real cave-carved world never would.
    fn terrain_and_caves(&self, x: i32, z: i32) -> Arc<Column> {
        let key = (self.seed, x, z);

        if let Some(column) = self.terrain_cave_cache.lock().unwrap().get(&key) {
            return column;
        }

        let mut column = self.build_terrain_column(x, z);
        CaveCarver::new(CAVE_RADIUS).carve(self.seed, x, z, &mut column, &self.block_ids);
        let column = Arc::new(column);

        self.terrain_cave_cache.lock().unwrap().insert(key, column.clone());
        column
    }

    /// Owner chunk `(owner_x, owner_z)`'s entire population (lakes through springs),
    /// computed once and cached.
    fn owner_population(&self, owner_x: i32, owner_z: i32) -> Arc<owner_buffer::OwnerBuffer> {
        let key = (self.seed, owner_x, owner_z);

        if let Some(buffer) = self.owner_population_cache.lock().unwrap().get(&key) {
            return buffer;
        }

        let buffer = Arc::new(population::populate_owner(self, owner_x, owner_z));
        self.owner_population_cache.lock().unwrap().insert(key, buffer.clone());
        buffer
    }

    /// Same as `owner_population`, but its home chunk is seeded from virgin terrain
    /// instead of backward neighbors. Exists so `owner_population` can ask "what
    /// would this backward neighbor produce alone" without recursing indefinitely.
    fn owner_population_isolated(&self, owner_x: i32, owner_z: i32) -> Arc<owner_buffer::OwnerBuffer> {
        let key = (self.seed, owner_x, owner_z);

        if let Some(buffer) = self.owner_population_isolated_cache.lock().unwrap().get(&key) {
            return buffer;
        }

        let buffer = Arc::new(population::populate_owner_isolated(self, owner_x, owner_z));
        self.owner_population_isolated_cache.lock().unwrap().insert(key, buffer.clone());
        buffer
    }

    /// Returns the column for `(x, z)`, building and caching it on first access. Several
    /// concurrent callers can race and each build their own copy on a cache miss - rare,
    /// and still correct - rather than holding the lock across the build itself.
    fn column(&self, x: i32, z: i32) -> Arc<Column> {
        let key = (self.seed, x, z);

        if let Some(column) = self.column_cache.lock().unwrap().get(&key) {
            return column.clone();
        }

        let column = Arc::new(self.build_column(x, z));
        self.column_cache.lock().unwrap().insert(key, column.clone());
        column
    }

    /// Forgets the cached column for `(x, z)`, once every subchunk that needed it has
    /// been generated.
    fn forget_column(&self, x: i32, z: i32) {
        self.column_cache.lock().unwrap().remove(&(self.seed, x, z));
    }

    /// Generates the subchunk at `(x, sub_y, z)`. This is the real entry point: it
    /// resolves the chunk's column from its cache (building it on first use) and slices
    /// out the requested 16-block band.
    pub(crate) fn generate_sub_chunk(&self, x: i32, sub_y: i8, z: i32) -> SubChunkBlocks {
        let column = self.column(x, z);

        let base_y = sub_y as usize * SUB_CHUNK_SIZE;
        let mut blocks = [[[0i32; SUB_CHUNK_SIZE]; SUB_CHUNK_SIZE]; SUB_CHUNK_SIZE];

        for (lx, plane) in blocks.iter_mut().enumerate() {
            for (ly, row) in plane.iter_mut().enumerate() {
                for (lz, block_id) in row.iter_mut().enumerate() {
                    *block_id = column.get(lx, base_y + ly, lz);
                }
            }
        }

        SubChunkBlocks { blocks }
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
        self.terrain_noise_2
            .sample_3d(&mut density_blend, world_offset_3d, DVec3::new(684.412 / 80.0, 684.412 / 160.0, 684.412 / 80.0));
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

    fn generate_terrain(&self, x: i32, z: i32, column: &mut Column, temperature_grid: &ClimateField, humidity_grid: &ClimateField, block_ids: &BlockIds) {
        let density = self.build_density_field(x, z, temperature_grid, humidity_grid);
        place_terrain(column, &density, temperature_grid, block_ids);
    }

    /// Samples the three noise fields `generate_surface` layers onto the ground:
    /// where sand/gravel patches show up, and how thick the surface layer is.
    fn sample_surface_fields(&self, x: i32, z: i32) -> (SurfaceField, SurfaceField, SurfaceField) {
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

        (sand_field, gravel_field, thickness_field)
    }

    fn generate_surface(&self, x: i32, z: i32, column: &mut Column, biome_grid: &BiomeGrid, rand: &mut JavaRand, block_ids: &BlockIds) {
        let (sand_field, gravel_field, thickness_field) = self.sample_surface_fields(x, z);

        // Iteration order (z outer, x inner) must match the reference generator exactly:
        // it determines how the per-column random calls below line up with world position.
        for lz in 0..CHUNK_WIDTH {
            for lx in 0..CHUNK_WIDTH {
                let biome = biome_grid[lx][lz];
                let has_sand = sand_field[lx][lz] + rand.next_double() * 0.2 > 0.0;
                let has_gravel = gravel_field[lx][lz] + rand.next_double() * 0.2 > 3.0;
                let surface_thickness = (thickness_field[lx][lz] / 3.0 + 3.0 + rand.next_double() * 0.25) as i32;

                carve_column(column, lx, lz, biome, has_sand, has_gravel, surface_thickness, rand, block_ids);
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
fn place_terrain(column: &mut Column, density: &DensityField, temperature_grid: &ClimateField, block_ids: &BlockIds) {
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
                                column.set(x, y, z, block_id);
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
fn carve_column(column: &mut Column, lx: usize, lz: usize, biome: Biome, has_sand: bool, has_gravel: bool, surface_thickness: i32, rand: &mut JavaRand, block_ids: &BlockIds) {
    let (biome_top_id, biome_filler_id) = match biome {
        Biome::Desert | Biome::IceDesert => (block_ids.sand, block_ids.sand),
        _ => (block_ids.grass, block_ids.dirt),
    };

    let mut top_id = biome_top_id;
    let mut filler_id = biome_filler_id;
    let mut remaining_thickness: i32 = -1;

    for y in (0..CHUNK_HEIGHT as i32).rev() {
        if y <= rand.next_i32_bounded(5) {
            column.set(lx, y as usize, lz, block_ids.bedrock);
            continue;
        }

        let prev_id = column.get(lx, y as usize, lz);

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
                    column.set(lx, y as usize, lz, top_id);
                } else {
                    column.set(lx, y as usize, lz, filler_id);
                }
            } else if remaining_thickness > 0 {
                column.set(lx, y as usize, lz, filler_id);

                remaining_thickness -= 1;
                if remaining_thickness == 0 && filler_id == block_ids.sand {
                    remaining_thickness = rand.next_i32_bounded(4);
                    filler_id = block_ids.sandstone;
                }
            }
        }
    }
}

/// Copies a generated subchunk's blocks into their absolute position in the real chunk.
fn insert_sub_chunk(chunk: &mut Chunk, sub_y: i8, sub_chunk: &SubChunkBlocks) {
    let base_y = sub_y as i32 * SUB_CHUNK_SIZE as i32;

    for (lx, plane) in sub_chunk.blocks.iter().enumerate() {
        for (ly, row) in plane.iter().enumerate() {
            for (lz, &block_id) in row.iter().enumerate() {
                chunk.set_block(lx as u8, base_y + ly as i32, lz as u8, 0, block_id);
            }
        }
    }
}

impl WorldGenerator for OverworldGenerator {
    fn generate(&self, _registry: &BlockRegistry, x: i32, z: i32, chunk: &mut Chunk) {
        for sub_y in 0..SUB_CHUNK_COUNT as i8 {
            let sub_chunk = self.generate_sub_chunk(x, sub_y, z);
            insert_sub_chunk(chunk, sub_y, &sub_chunk);
        }

        // Every subchunk of this column has now been requested, so its cache entry can
        // be freed. Once subchunks are requested independently instead of all-at-once
        // here, eviction will need to move to whatever signals a chunk is complete.
        self.forget_column(x, z);
    }
}
