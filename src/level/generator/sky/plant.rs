use super::SkyGenerator;
use super::quad_chunk_buffer::QuadChunkBuffer;
use crate::level::generator::overworld::biome::Biome;
use crate::level::generator::shared::plant::{place_cactus, place_plants, place_pumpkin, place_sugar_canes};
use crate::level::generator::shared::vein::next_offset;
use crate::rand::java::JavaRand;
use crate::rand::primitives::Bound;
use glam::IVec3;

pub fn populate_from(generator: &SkyGenerator, buffer: &mut QuadChunkBuffer, owner_x: i32, owner_z: i32, rand: &mut JavaRand) {
    let origin = IVec3::new(owner_x * super::CHUNK_WIDTH as i32, 0, owner_z * super::CHUNK_WIDTH as i32);
    let biome = generator.biome_at(origin.x + 16, origin.z + 16);
    let block_ids = &generator.block_ids;

    for _ in 0..2 {
        let pos = origin + next_offset(rand, 128, 8);
        place_plants(generator, buffer, pos, block_ids, block_ids.dandelion, 64, false, rand, |block_ids, below| {
            below == block_ids.grass || below == block_ids.dirt
        });
    }

    if rand.random_with::<i32>(Bound::new(2)) == 0 {
        let pos = origin + next_offset(rand, 128, 8);
        place_plants(generator, buffer, pos, block_ids, block_ids.poppy, 64, false, rand, |block_ids, below| {
            below == block_ids.grass || below == block_ids.dirt
        });
    }

    if rand.random_with::<i32>(Bound::new(4)) == 0 {
        let pos = origin + next_offset(rand, 128, 8);
        place_plants(generator, buffer, pos, block_ids, block_ids.brown_mushroom, 64, false, rand, |block_ids, below| {
            below != block_ids.air && below != block_ids.water
        });
    }

    if rand.random_with::<i32>(Bound::new(8)) == 0 {
        let pos = origin + next_offset(rand, 128, 8);
        place_plants(generator, buffer, pos, block_ids, block_ids.red_mushroom, 64, false, rand, |block_ids, below| {
            below != block_ids.air && below != block_ids.water
        });
    }

    for _ in 0..10 {
        let pos = origin + next_offset(rand, 128, 8);
        place_sugar_canes(generator, buffer, pos, block_ids, rand);
    }

    if rand.random_with::<i32>(Bound::new(32)) == 0 {
        let pos = origin + next_offset(rand, 128, 8);
        place_pumpkin(generator, buffer, pos, block_ids, rand);
    }

    if biome == Biome::Desert {
        for _ in 0..10 {
            let pos = origin + next_offset(rand, 128, 8);
            place_cactus(generator, buffer, pos, block_ids, rand);
        }
    }
}
