pub mod block_ids;
pub mod cave;
pub mod chunk_buffer;
pub mod dungeon;
pub mod lake;
pub mod plant;
pub mod quad_chunk_buffer;
pub mod snow;
pub mod spring;
pub mod tree;
pub mod vein;

use glam::IVec3;

use crate::rand::java::JavaRand;

pub const HORIZONTAL_FACES: [IVec3; 4] = [IVec3::new(0, 0, -1), IVec3::new(0, 0, 1), IVec3::new(-1, 0, 0), IVec3::new(1, 0, 0)];

pub const CHUNK_WIDTH: usize = 16;
pub const CHUNK_HEIGHT: usize = 128;
pub const SUB_CHUNK_SIZE: usize = 16;
pub const SUB_CHUNK_COUNT: usize = CHUNK_HEIGHT / SUB_CHUNK_SIZE;
pub const CAVE_RADIUS: i32 = 8;
pub const SNOW_TEMPERATURE_REFERENCE_HEIGHT: i32 = 64;

pub fn chunk_seed(world_seed: i64, x: i32, z: i32) -> i64 {
    let mut rand = JavaRand::new(world_seed);
    let x_mul = rand.random::<i64>().wrapping_div(2).wrapping_mul(2).wrapping_add(1);
    let z_mul = rand.random::<i64>().wrapping_div(2).wrapping_mul(2).wrapping_add(1);
    i64::wrapping_add((x as i64).wrapping_mul(x_mul), (z as i64).wrapping_mul(z_mul)) ^ world_seed
}
