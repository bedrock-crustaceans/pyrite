use glam::IVec3;

use super::block_ids::BlockIds;
use super::quad_chunk_buffer::{QuadChunkBuffer, read, write};
use super::tree::is_leaves;
use super::{HORIZONTAL_FACES, TerrainSource};
use crate::rand::java::JavaRand;
use crate::rand::primitives::Bound;

#[allow(clippy::too_many_arguments)]
pub fn place_plants<T: TerrainSource>(
    generator: &T,
    buffer: &mut QuadChunkBuffer,
    mut pos: IVec3,
    block_ids: &BlockIds,
    plant_id: i32,
    count: i32,
    find_ground: bool,
    rand: &mut JavaRand,
    is_valid_support: fn(block_ids: &BlockIds, support: i32) -> bool,
) {
    if find_ground {
        while pos.y > 0 {
            let id = read(generator, buffer, pos.x, pos.y, pos.z);
            if id != block_ids.air && !is_leaves(block_ids, id) {
                break;
            }
            pos.y -= 1;
        }
    }

    for _ in 0..count {
        let place_pos = pos
            + IVec3::new(
                rand.random_with::<i32>(Bound::new(8)) - rand.random_with::<i32>(Bound::new(8)),
                rand.random_with::<i32>(Bound::new(4)) - rand.random_with::<i32>(Bound::new(4)),
                rand.random_with::<i32>(Bound::new(8)) - rand.random_with::<i32>(Bound::new(8)),
            );

        if read(generator, buffer, place_pos.x, place_pos.y, place_pos.z) != block_ids.air {
            continue;
        }

        let support = read(generator, buffer, place_pos.x, place_pos.y - 1, place_pos.z);
        if !is_valid_support(block_ids, support) {
            continue;
        }

        write(buffer, place_pos.x, place_pos.y, place_pos.z, plant_id);
    }
}

pub fn place_sugar_canes(generator: &impl TerrainSource, buffer: &mut QuadChunkBuffer, pos: IVec3, block_ids: &BlockIds, rand: &mut JavaRand) {
    for _ in 0..20 {
        let place_pos = pos
            + IVec3::new(
                rand.random_with::<i32>(Bound::new(4)) - rand.random_with::<i32>(Bound::new(4)),
                0,
                rand.random_with::<i32>(Bound::new(4)) - rand.random_with::<i32>(Bound::new(4)),
            );

        if read(generator, buffer, place_pos.x, place_pos.y, place_pos.z) != block_ids.air {
            continue;
        }

        let below_pos = place_pos - IVec3::Y;

        for face in HORIZONTAL_FACES {
            let water_pos = below_pos + face;
            if read(generator, buffer, water_pos.x, water_pos.y, water_pos.z) != block_ids.water {
                continue;
            }

            let v = rand.random_with::<i32>(Bound::new(3)) + 1;
            let height = rand.random_with::<i32>(Bound::new(v)) + 2;

            let below = read(generator, buffer, below_pos.x, below_pos.y, below_pos.z);
            let can_place = (below == block_ids.grass || below == block_ids.dirt)
                && HORIZONTAL_FACES.iter().any(|&f| {
                    let p = below_pos + f;
                    read(generator, buffer, p.x, p.y, p.z) == block_ids.water
                });

            if can_place {
                for dy in 0..height {
                    write(buffer, place_pos.x, place_pos.y + dy, place_pos.z, block_ids.reeds);
                }
            }
        }
    }
}

pub fn place_pumpkin(generator: &impl TerrainSource, buffer: &mut QuadChunkBuffer, pos: IVec3, block_ids: &BlockIds, rand: &mut JavaRand) {
    for _ in 0..64 {
        let place_pos = pos
            + IVec3::new(
                rand.random_with::<i32>(Bound::new(8)) - rand.random_with::<i32>(Bound::new(8)),
                rand.random_with::<i32>(Bound::new(4)) - rand.random_with::<i32>(Bound::new(4)),
                rand.random_with::<i32>(Bound::new(8)) - rand.random_with::<i32>(Bound::new(8)),
            );

        if read(generator, buffer, place_pos.x, place_pos.y, place_pos.z) != block_ids.air {
            continue;
        }
        let below = read(generator, buffer, place_pos.x, place_pos.y - 1, place_pos.z);
        if below != block_ids.grass {
            continue;
        }

        // Facing isn't modeled (no permutation-state writer exists yet for any block
        // in this generator), but the roll is still consumed to keep the RNG stream
        // in sync with the reference.
        let _facing = rand.random_with::<i32>(Bound::new(4));
        write(buffer, place_pos.x, place_pos.y, place_pos.z, block_ids.pumpkin);
    }
}

pub fn place_cactus(generator: &impl TerrainSource, buffer: &mut QuadChunkBuffer, pos: IVec3, block_ids: &BlockIds, rand: &mut JavaRand) {
    for _ in 0..10 {
        let place_pos = pos
            + IVec3::new(
                rand.random_with::<i32>(Bound::new(8)) - rand.random_with::<i32>(Bound::new(8)),
                rand.random_with::<i32>(Bound::new(4)) - rand.random_with::<i32>(Bound::new(4)),
                rand.random_with::<i32>(Bound::new(8)) - rand.random_with::<i32>(Bound::new(8)),
            );

        if read(generator, buffer, place_pos.x, place_pos.y, place_pos.z) != block_ids.air {
            continue;
        }

        let v = rand.random_with::<i32>(Bound::new(3)) + 1;
        let height = rand.random_with::<i32>(Bound::new(v)) + 1;

        for dy in 0..height {
            if can_place_cactus(generator, buffer, block_ids, place_pos) {
                write(buffer, place_pos.x, place_pos.y + dy, place_pos.z, block_ids.cactus);
            }
        }
    }
}

fn can_place_cactus(generator: &impl TerrainSource, buffer: &QuadChunkBuffer, block_ids: &BlockIds, pos: IVec3) -> bool {
    for face in HORIZONTAL_FACES {
        let neighbor = pos + face;
        let id = read(generator, buffer, neighbor.x, neighbor.y, neighbor.z);
        if id != block_ids.air && id != block_ids.water {
            return false;
        }
    }
    let below = read(generator, buffer, pos.x, pos.y - 1, pos.z);
    below == block_ids.cactus || below == block_ids.sand
}
