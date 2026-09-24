use glam::IVec3;

use super::block_ids::BlockIds;
use super::quad_chunk_buffer::{QuadChunkBuffer, read, write};
use super::{CHUNK_HEIGHT, CHUNK_WIDTH, ClimateSource, SNOW_TEMPERATURE_REFERENCE_HEIGHT, TerrainSource};

pub fn populate_from<G: TerrainSource + ClimateSource>(generator: &G, buffer: &mut QuadChunkBuffer, owner_x: i32, owner_z: i32, block_ids: &BlockIds) {
    let origin = IVec3::new(owner_x * CHUNK_WIDTH as i32, 0, owner_z * CHUNK_WIDTH as i32);

    for dx in 0..CHUNK_WIDTH as i32 {
        let wx = origin.x + 8 + dx;
        for dz in 0..CHUNK_WIDTH as i32 {
            let wz = origin.z + 8 + dz;

            let Some(height) = top_solid_or_liquid_height(generator, buffer, block_ids, wx, wz) else {
                continue;
            };
            if !(1..CHUNK_HEIGHT as i32).contains(&height) {
                continue;
            }

            let temperature = generator.climate_at(wx, wz).0;
            let adjusted_temperature = temperature - (height - SNOW_TEMPERATURE_REFERENCE_HEIGHT) as f64 / 64.0 * 0.3;
            if adjusted_temperature >= 0.5 {
                continue;
            }

            if read(generator, buffer, wx, height, wz) != block_ids.air {
                continue;
            }

            let below = read(generator, buffer, wx, height - 1, wz);
            if !is_solid_ground(block_ids, below) {
                continue;
            }

            write(buffer, wx, height, wz, block_ids.snow_layer);
        }
    }
}

fn top_solid_or_liquid_height(generator: &impl TerrainSource, buffer: &QuadChunkBuffer, block_ids: &BlockIds, wx: i32, wz: i32) -> Option<i32> {
    for wy in (0..CHUNK_HEIGHT as i32).rev() {
        let id = read(generator, buffer, wx, wy, wz);
        if id != block_ids.air && !is_small_plant(block_ids, id) {
            return Some(wy + 1);
        }
    }
    None
}

fn is_small_plant(block_ids: &BlockIds, id: i32) -> bool {
    id == block_ids.dandelion
        || id == block_ids.poppy
        || id == block_ids.tall_grass
        || id == block_ids.fern
        || id == block_ids.deadbush
        || id == block_ids.red_mushroom
        || id == block_ids.brown_mushroom
        || id == block_ids.reeds
}

fn is_solid_ground(block_ids: &BlockIds, id: i32) -> bool {
    id != block_ids.air && id != block_ids.water && id != block_ids.water_flowing && id != block_ids.lava && id != block_ids.lava_still && id != block_ids.ice && !is_small_plant(block_ids, id)
}
