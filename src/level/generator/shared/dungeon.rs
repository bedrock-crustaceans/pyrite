use glam::IVec3;

use super::block_ids::BlockIds;
use super::quad_chunk_buffer::{QuadChunkBuffer, read, write};
use super::vein::next_offset;
use super::{CHUNK_WIDTH, HORIZONTAL_FACES, TerrainSource};
use crate::rand::java::JavaRand;
use crate::rand::primitives::Bound;

pub fn populate_from(generator: &impl TerrainSource, buffer: &mut QuadChunkBuffer, owner_x: i32, owner_z: i32, block_ids: &BlockIds, rand: &mut JavaRand) {
    let origin = IVec3::new(owner_x * CHUNK_WIDTH as i32, 0, owner_z * CHUNK_WIDTH as i32);

    for _ in 0..8 {
        let pos = origin + next_offset(rand, 128, 8);
        place_dungeon(generator, buffer, block_ids, pos, rand);
    }
}

const CHEST_SLOT_COUNT: i32 = 27;

fn is_solid(block_ids: &BlockIds, id: i32) -> bool {
    id != block_ids.air && id != block_ids.water && id != block_ids.lava && id != block_ids.lava_still
}

fn roll_chest_stack(rand: &mut JavaRand) -> bool {
    match rand.random_with::<i32>(Bound::new(11)) {
        0 => true,
        1 => {
            rand.random_with::<i32>(Bound::new(4));
            true
        }
        2 => true,
        3 => true,
        4 => {
            rand.random_with::<i32>(Bound::new(4));
            true
        }
        5 => {
            rand.random_with::<i32>(Bound::new(4));
            true
        }
        6 => true,
        7 => rand.random_with::<i32>(Bound::new(100)) == 0,
        8 => {
            if rand.random_with::<i32>(Bound::new(2)) == 0 {
                rand.random_with::<i32>(Bound::new(4));
                true
            } else {
                false
            }
        }
        9 => {
            if rand.random_with::<i32>(Bound::new(10)) == 0 {
                rand.random_with::<i32>(Bound::new(2));
                true
            } else {
                false
            }
        }
        10 => true,
        _ => false,
    }
}

fn place_dungeon(generator: &impl TerrainSource, buffer: &mut QuadChunkBuffer, block_ids: &BlockIds, pos: IVec3, rand: &mut JavaRand) -> bool {
    let x_radius = rand.random_with::<i32>(Bound::new(2)) + 2;
    let z_radius = rand.random_with::<i32>(Bound::new(2)) + 2;
    let height = 3;
    let mut air_count = 0i32;

    let start = pos - IVec3::new(x_radius + 1, 1, x_radius + 1);
    let end = pos + IVec3::new(x_radius + 1, height + 1, x_radius + 1);

    for wx in start.x..=end.x {
        for wy in start.y..=end.y {
            for wz in start.z..=end.z {
                let id = read(generator, buffer, wx, wy, wz);
                let solid = is_solid(block_ids, id);

                if (wy == start.y || wy == end.y) && !solid {
                    return false;
                } else if wy == pos.y && (wx == start.x || wx == end.x || wz == start.z || wz == end.z) {
                    let above_id = read(generator, buffer, wx, wy + 1, wz);
                    if id == block_ids.air && above_id == block_ids.air {
                        air_count += 1;
                    }
                }
            }
        }
    }

    if !(1..=5).contains(&air_count) {
        return false;
    }

    // Carve the dungeon and fill walls.
    for wx in start.x..=end.x {
        for wy in (start.y..end.y).rev() {
            for wz in start.z..=end.z {
                if wx != start.x && wy != start.y && wz != start.z && wx != end.x && wz != end.z {
                    write(buffer, wx, wy, wz, block_ids.air);
                } else {
                    let below_id = read(generator, buffer, wx, wy - 1, wz);
                    if wy >= 0 && !is_solid(block_ids, below_id) {
                        write(buffer, wx, wy, wz, block_ids.air);
                    } else {
                        let here_id = read(generator, buffer, wx, wy, wz);
                        if is_solid(block_ids, here_id) {
                            if wy == start.y && rand.random_with::<i32>(Bound::new(4)) != 0 {
                                write(buffer, wx, wy, wz, block_ids.mossy_cobblestone);
                            } else {
                                write(buffer, wx, wy, wz, block_ids.cobblestone);
                            }
                        }
                    }
                }
            }
        }
    }

    // Place chests.
    for _ in 0..2 {
        'chest_try: for _ in 0..3 {
            let bound = z_radius * 2 + 1;
            let bound1 = x_radius * 2 + 1;
            let chest_pos = pos + IVec3::new(rand.random_with::<i32>(Bound::new(bound1)) - x_radius, 0, rand.random_with::<i32>(Bound::new(bound)) - z_radius);

            if read(generator, buffer, pos.x, pos.y, pos.z) == block_ids.air {
                let mut solid_count = 0;
                for face in HORIZONTAL_FACES {
                    let neighbor = chest_pos + face;
                    if is_solid(block_ids, read(generator, buffer, neighbor.x, neighbor.y, neighbor.z)) {
                        solid_count += 1;
                        if solid_count > 1 {
                            continue 'chest_try;
                        }
                    }
                }

                if solid_count == 0 {
                    continue 'chest_try;
                }

                for _ in 0..8 {
                    if roll_chest_stack(rand) {
                        rand.random_with::<i32>(Bound::new(CHEST_SLOT_COUNT));
                    }
                }

                write(buffer, chest_pos.x, chest_pos.y, chest_pos.z, block_ids.chest);
                break;
            }
        }
    }

    let _entity_kind = rand.random_with::<i32>(Bound::new(4));
    write(buffer, pos.x, pos.y, pos.z, block_ids.mob_spawner);

    true
}
