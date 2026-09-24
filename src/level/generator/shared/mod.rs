//! Infrastructure and population features shared by more than one dimension's
//! generator. Only genuinely identical (or near-identical, parameterized) logic lives
//! here - each dimension keeps its own terrain/surface generation and whichever
//! population features it actually diverges on.

pub mod block_ids;
pub mod cave;
pub mod chunk_buffer;
pub mod dungeon;
pub mod lake;
pub mod quad_chunk_buffer;
pub mod plant;
pub mod population;
pub mod snow;
pub mod spring;
pub mod tree;
pub mod vein;

use std::collections::{HashMap, VecDeque};
use std::sync::Arc;

use chorus::level::chunk::Chunk;
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
pub const TERRAIN_CAVE_CACHE_CAPACITY: usize = 256;
pub const SNOW_TEMPERATURE_REFERENCE_HEIGHT: i32 = 64;

pub trait TerrainSource {
    fn terrain_and_caves(&self, x: i32, z: i32) -> Arc<ChunkBuffer>;
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

pub struct BoundedCache<V> {
    capacity: usize,
    order: VecDeque<(i64, i32, i32)>,
    entries: HashMap<(i64, i32, i32), Arc<V>>,
}

impl<V> BoundedCache<V> {
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity,
            order: VecDeque::new(),
            entries: HashMap::new(),
        }
    }

    pub fn get(&self, key: &(i64, i32, i32)) -> Option<Arc<V>> {
        self.entries.get(key).cloned()
    }

    pub fn insert(&mut self, key: (i64, i32, i32), value: Arc<V>) {
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

pub fn insert_sub_chunk(chunk: &mut Chunk, sub_y: i8, sub_chunk: &SubChunkBlocks) {
    let base_y = sub_y as i32 * SUB_CHUNK_SIZE as i32;

    for (lx, plane) in sub_chunk.blocks.iter().enumerate() {
        for (ly, row) in plane.iter().enumerate() {
            for (lz, &block_id) in row.iter().enumerate() {
                chunk.set_block(lx as u8, base_y + ly as i32, lz as u8, 0, block_id);
            }
        }
    }
}
