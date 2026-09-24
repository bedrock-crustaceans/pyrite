use glam::IVec3;

use super::block_ids::BlockIds;
use super::quad_chunk_buffer::{QuadChunkBuffer, read, write};
use super::{CHUNK_WIDTH, HORIZONTAL_FACES, TerrainSource};
use crate::rand::java::JavaRand;
use crate::rand::primitives::Bound;

pub fn populate_from(generator: &impl TerrainSource, buffer: &mut QuadChunkBuffer, owner_x: i32, owner_z: i32, block_ids: &BlockIds, rand: &mut JavaRand) {
    let origin = IVec3::new(owner_x * CHUNK_WIDTH as i32, 0, owner_z * CHUNK_WIDTH as i32);

    for _ in 0..50 {
        let pos = origin
            + IVec3::new(
                rand.random_with::<i32>(Bound::new(CHUNK_WIDTH as i32)) + 8,
                {
                    let v = rand.random_with::<i32>(Bound::new(120));
                    rand.random_with::<i32>(Bound::new(v + 8))
                },
                rand.random_with::<i32>(Bound::new(CHUNK_WIDTH as i32)) + 8,
            );
        place_spring(generator, buffer, block_ids, block_ids.water_flowing, pos);
    }

    for _ in 0..20 {
        let pos = origin
            + IVec3::new(
                rand.random_with::<i32>(Bound::new(CHUNK_WIDTH as i32)) + 8,
                {
                    let v = rand.random_with::<i32>(Bound::new(112));
                    let v = rand.random_with::<i32>(Bound::new(v + 8));
                    rand.random_with::<i32>(Bound::new(v + 8))
                },
                rand.random_with::<i32>(Bound::new(CHUNK_WIDTH as i32)) + 8,
            );
        place_spring(generator, buffer, block_ids, block_ids.lava, pos);
    }
}

fn place_spring(generator: &impl TerrainSource, buffer: &mut QuadChunkBuffer, block_ids: &BlockIds, fluid_id: i32, pos: IVec3) {
    if read(generator, buffer, pos.x, pos.y + 1, pos.z) != block_ids.stone {
        return;
    }
    if read(generator, buffer, pos.x, pos.y - 1, pos.z) != block_ids.stone {
        return;
    }

    let here = read(generator, buffer, pos.x, pos.y, pos.z);
    if here != block_ids.air && here != block_ids.stone {
        return;
    }

    let mut stone_count = 0;
    let mut air_count = 0;
    for face in HORIZONTAL_FACES {
        let neighbor = pos + face;
        let id = read(generator, buffer, neighbor.x, neighbor.y, neighbor.z);
        if id == block_ids.stone {
            stone_count += 1;
        } else if id == block_ids.air {
            air_count += 1;
        }
    }

    if stone_count == 3 && air_count == 1 {
        write(buffer, pos.x, pos.y, pos.z, fluid_id);
    }
}
