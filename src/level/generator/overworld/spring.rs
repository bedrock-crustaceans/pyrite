use glam::IVec3;

use crate::level::generator::java_rand::JavaRand;

use super::owner_buffer::{OwnerBuffer, read, write};
use super::{BlockIds, CHUNK_WIDTH, OverworldGenerator};

const HORIZONTAL_FACES: [IVec3; 4] = [IVec3::new(0, 0, -1), IVec3::new(0, 0, 1), IVec3::new(-1, 0, 0), IVec3::new(1, 0, 0)];

/// Water/lava spring sources - chorus has no flowing-liquid physics yet, so these
/// won't animate, but the source block itself still generates.
pub fn populate_from(generator: &OverworldGenerator, buffer: &mut OwnerBuffer, owner_x: i32, owner_z: i32, rand: &mut JavaRand) {
    let origin = IVec3::new(owner_x * CHUNK_WIDTH as i32, 0, owner_z * CHUNK_WIDTH as i32);
    let block_ids = &generator.block_ids;

    for _ in 0..50 {
        let pos = origin
            + IVec3::new(
                rand.next_i32_bounded(CHUNK_WIDTH as i32) + 8,
                {
                    let v = rand.next_i32_bounded(120);
                    rand.next_i32_bounded(v + 8)
                },
                rand.next_i32_bounded(CHUNK_WIDTH as i32) + 8,
            );
        place_spring(generator, buffer, block_ids, block_ids.water_flowing, pos);
    }

    for _ in 0..20 {
        let pos = origin
            + IVec3::new(
                rand.next_i32_bounded(CHUNK_WIDTH as i32) + 8,
                {
                    let v = rand.next_i32_bounded(112);
                    let v = rand.next_i32_bounded(v + 8);
                    rand.next_i32_bounded(v + 8)
                },
                rand.next_i32_bounded(CHUNK_WIDTH as i32) + 8,
            );
        place_spring(generator, buffer, block_ids, block_ids.lava, pos);
    }
}

/// Places a spring at `pos` if it's sandwiched between stone above/below, is itself
/// air or stone, and exactly 3 of its 4 horizontal neighbors are stone with the 4th
/// open. Never touches RNG, so a failed attempt can't desync anything downstream.
fn place_spring(generator: &OverworldGenerator, buffer: &mut OwnerBuffer, block_ids: &BlockIds, fluid_id: i32, pos: IVec3) {
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
