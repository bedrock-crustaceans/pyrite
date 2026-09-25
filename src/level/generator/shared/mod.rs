//! Infrastructure and population features shared by more than one dimension's
//! generator. Only genuinely identical (or near-identical, parameterized) logic lives
//! here - each dimension keeps its own terrain/surface generation and whichever
//! population features it actually diverges on.

pub mod block_ids;
pub mod cave;
pub mod chunk_buffer;
pub mod dungeon;
pub mod lake;
pub mod phases;
pub mod plant;
pub mod population;
pub mod quad_chunk_buffer;
pub mod snow;
pub mod spring;
pub mod tree;
pub mod vein;

use std::sync::Arc;

use chorus::level::chunk::Chunk;
use chorus::level::sub_chunk::SubChunk;
use glam::IVec3;

use crate::level::generator::overworld::biome::Biome;
use crate::rand::java::JavaRand;
use chunk_buffer::ChunkBuffer;

pub const HORIZONTAL_FACES: [IVec3; 4] = [IVec3::new(0, 0, -1), IVec3::new(0, 0, 1), IVec3::new(-1, 0, 0), IVec3::new(1, 0, 0)];

pub const CHUNK_WIDTH: usize = 16;
pub const CHUNK_HEIGHT: usize = 128;
pub const SUB_CHUNK_SIZE: usize = 16;
pub const SUB_CHUNK_COUNT: usize = CHUNK_HEIGHT / SUB_CHUNK_SIZE;
pub const CAVE_RADIUS: i32 = 8;
pub const SNOW_TEMPERATURE_REFERENCE_HEIGHT: i32 = 64;

pub trait TerrainSource {
    /// Terrain shape only - density fields, surface - with no caves carved into it yet.
    fn raw_terrain(&self, x: i32, z: i32) -> ChunkBuffer;
    /// Carves caves into an already-built terrain column, in place.
    fn carve_caves(&self, x: i32, z: i32, column: &mut ChunkBuffer);

    /// The column decoration actually reads: terrain with caves carved into it. A default method
    /// composing the two above - a dimension only implements the two building blocks, not this.
    fn terrain_and_caves(&self, x: i32, z: i32) -> Arc<ChunkBuffer> {
        let mut column = self.raw_terrain(x, z);
        self.carve_caves(x, z, &mut column);
        Arc::new(column)
    }

    fn min_sub_chunk_y(&self) -> i8;
    fn dimension_sub_chunk_count(&self) -> usize;
    fn air_id(&self) -> i32;
    fn biome(&self) -> i32;
}

pub trait ClimateSource {
    fn climate_at(&self, x: i32, z: i32) -> (f64, f64, Biome);
    fn feature_noise_at(&self, x: f64, z: f64) -> f64;

    fn biome_at(&self, x: i32, z: i32) -> Biome {
        self.climate_at(x, z).2
    }
}

pub fn chunk_seed(world_seed: i64, x: i32, z: i32) -> i64 {
    let mut rand = JavaRand::new(world_seed);
    let x_mul = rand.random::<i64>().wrapping_div(2).wrapping_mul(2).wrapping_add(1);
    let z_mul = rand.random::<i64>().wrapping_div(2).wrapping_mul(2).wrapping_add(1);
    i64::wrapping_add((x as i64).wrapping_mul(x_mul), (z as i64).wrapping_mul(z_mul)) ^ world_seed
}

pub struct SubChunkBlocks {
    pub blocks: [[[i32; SUB_CHUNK_SIZE]; SUB_CHUNK_SIZE]; SUB_CHUNK_SIZE],
}

/// Bulk-loads a whole sub-chunk's worth of blocks in one shot. Building the palette via 4096
/// individual `chunk.set_block()` calls (going through `SubChunk::set()`'s incremental,
/// in-game-edit-oriented path each time) is measurably more expensive than this for freshly
/// generated data - see `Palette::from_blocks`.
pub fn insert_sub_chunk(chunk: &mut Chunk, sub_y: i8, sub_chunk: &SubChunkBlocks, air_id: i32, biome: i32) {
    let mut blocks = [0i32; 4096];
    for (lx, plane) in sub_chunk.blocks.iter().enumerate() {
        for (ly, row) in plane.iter().enumerate() {
            for (lz, &block_id) in row.iter().enumerate() {
                blocks[SubChunk::index(lx as u8, ly as u8, lz as u8)] = block_id;
            }
        }
    }

    if let Some(existing) = chunk.get_sub_chunk_mut(sub_y) {
        *existing = SubChunk::from_blocks(&blocks, air_id, biome);
    }
}
