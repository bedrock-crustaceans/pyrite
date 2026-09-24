use glam::IVec3;

use crate::level::generator::shared::quad_chunk_buffer::{QuadChunkBuffer, read};
use crate::level::generator::shared::vein::{next_offset, place_vein, populate_ores};
use crate::rand::java::JavaRand;

use super::{CHUNK_WIDTH, OverworldGenerator};

pub fn populate_from(generator: &OverworldGenerator, buffer: &mut QuadChunkBuffer, owner_x: i32, owner_z: i32, rand: &mut JavaRand) {
    let block_ids = &generator.block_ids;
    let origin = IVec3::new(owner_x * CHUNK_WIDTH as i32, 0, owner_z * CHUNK_WIDTH as i32);

    for _ in 0..10 {
        let pos = origin + next_offset(rand, 128, 0);
        if read(generator, buffer, pos.x, pos.y, pos.z) == block_ids.water {
            place_vein(generator, buffer, block_ids.sand, block_ids.clay, 32, pos, rand);
        }
    }

    populate_ores(generator, buffer, owner_x, owner_z, block_ids, rand);
}
