use super::OverworldGenerator;
use super::biome::Biome;
use crate::level::generator::shared::plant::{PlantRule, place_cactus, place_plants, place_pumpkin, place_sugar_canes};
use crate::level::generator::shared::quad_chunk_buffer::QuadChunkBuffer;
use crate::level::generator::shared::vein::next_offset;
use crate::rand::java::JavaRand;
use crate::rand::primitives::Bound;
use glam::IVec3;

pub fn populate_from(generator: &OverworldGenerator, buffer: &mut QuadChunkBuffer, owner_x: i32, owner_z: i32, rand: &mut JavaRand) {
    let origin = IVec3::new(owner_x * super::CHUNK_WIDTH as i32, 0, owner_z * super::CHUNK_WIDTH as i32);
    let biome = generator.biome_at(origin.x + 16, origin.z + 16);
    let block_ids = &generator.block_ids;

    let dandelion_count = match biome {
        Biome::Forest | Biome::Taiga => 2,
        Biome::SeasonalForest => 4,
        Biome::Plains => 3,
        _ => 0,
    };

    for _ in 0..dandelion_count {
        let pos = origin + next_offset(rand, 128, 8);
        place_plants(buffer, pos, block_ids, block_ids.dandelion, 64, false, PlantRule::Flower, rand);
    }

    let tall_grass_count = match biome {
        Biome::Forest => 2,
        Biome::RainForest => 10,
        Biome::SeasonalForest => 2,
        Biome::Taiga => 1,
        Biome::Plains => 10,
        _ => 0,
    };

    for _ in 0..tall_grass_count {
        let mut plant_id = block_ids.tall_grass;
        if biome == Biome::RainForest && rand.random_with::<i32>(Bound::new(3)) != 0 {
            plant_id = block_ids.fern;
        }
        let pos = origin + next_offset(rand, 128, 8);
        place_plants(buffer, pos, block_ids, plant_id, 128, true, PlantRule::Flower, rand);
    }

    if biome == Biome::Desert {
        for _ in 0..2 {
            let pos = origin + next_offset(rand, 128, 8);
            place_plants(buffer, pos, block_ids, block_ids.deadbush, 4, true, PlantRule::DeadBush, rand);
        }
    }

    if rand.random_with::<i32>(Bound::new(2)) == 0 {
        let pos = origin + next_offset(rand, 128, 8);
        place_plants(buffer, pos, block_ids, block_ids.poppy, 64, false, PlantRule::Flower, rand);
    }

    if rand.random_with::<i32>(Bound::new(4)) == 0 {
        let pos = origin + next_offset(rand, 128, 8);
        place_plants(buffer, pos, block_ids, block_ids.brown_mushroom, 64, false, PlantRule::Mushroom, rand);
    }

    if rand.random_with::<i32>(Bound::new(8)) == 0 {
        let pos = origin + next_offset(rand, 128, 8);
        place_plants(buffer, pos, block_ids, block_ids.red_mushroom, 64, false, PlantRule::Mushroom, rand);
    }

    for _ in 0..10 {
        let pos = origin + next_offset(rand, 128, 8);
        place_sugar_canes(buffer, pos, block_ids, rand);
    }

    if rand.random_with::<i32>(Bound::new(32)) == 0 {
        let pos = origin + next_offset(rand, 128, 8);
        place_pumpkin(buffer, pos, block_ids, rand);
    }

    if biome == Biome::Desert {
        for _ in 0..10 {
            let pos = origin + next_offset(rand, 128, 8);
            place_cactus(buffer, pos, block_ids, rand);
        }
    }
}
