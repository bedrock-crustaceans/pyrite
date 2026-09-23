use glam::IVec3;

use crate::level::generator::java_rand::JavaRand;

use super::owner_buffer::{OwnerBuffer, read, write};
use super::vein::next_offset;
use super::{BlockIds, CHUNK_WIDTH, OverworldGenerator};

/// Places every dungeon belonging to owner chunk `(owner_x, owner_z)` into its own
/// `OwnerBuffer`. Continues the same population RNG stream `rand` is already partway
/// through (see `population::populate_owner`) - 8 attempts per owner, matching the
/// reference.
pub fn populate_from(generator: &OverworldGenerator, buffer: &mut OwnerBuffer, owner_x: i32, owner_z: i32, rand: &mut JavaRand) {
    let origin = IVec3::new(owner_x * CHUNK_WIDTH as i32, 0, owner_z * CHUNK_WIDTH as i32);
    let block_ids = &generator.block_ids;

    for _ in 0..8 {
        let pos = origin + next_offset(rand, 128, 8);
        place_dungeon(generator, buffer, block_ids, pos, rand);
    }
}

/// Matches the reference's `Face::HORIZONTAL` order - see `plant.rs`'s copy of the
/// same constant.
const HORIZONTAL_FACES: [IVec3; 4] = [IVec3::new(0, 0, -1), IVec3::new(0, 0, 1), IVec3::new(-1, 0, 0), IVec3::new(1, 0, 0)];

const CHEST_SLOT_COUNT: i32 = 27;

fn is_solid(block_ids: &BlockIds, id: i32) -> bool {
    id != block_ids.air && id != block_ids.water && id != block_ids.lava && id != block_ids.lava_still
}

/// Replicates `gen_chest_stack`'s exact RNG consumption without constructing an item
/// (chorus has no inventory system yet). Returns whether the roll was non-empty,
/// since that decides whether a slot-index roll follows.
fn roll_chest_stack(rand: &mut JavaRand) -> bool {
    match rand.next_i32_bounded(11) {
        0 => true,
        1 => {
            rand.next_i32_bounded(4);
            true
        }
        2 => true,
        3 => true,
        4 => {
            rand.next_i32_bounded(4);
            true
        }
        5 => {
            rand.next_i32_bounded(4);
            true
        }
        6 => true,
        7 => rand.next_i32_bounded(100) == 0,
        8 => {
            if rand.next_i32_bounded(2) == 0 {
                rand.next_i32_bounded(4);
                true
            } else {
                false
            }
        }
        9 => {
            if rand.next_i32_bounded(10) == 0 {
                rand.next_i32_bounded(2);
                true
            } else {
                false
            }
        }
        10 => true,
        _ => false,
    }
}

/// Places a dungeon (spawner room + up to 2 chests) centered at `pos`, ported from the
/// reference's `DungeonGenerator`. Chest and item contents are rolled (to keep the RNG
/// stream in sync with the reference) but never actually constructed or attached to
/// the chest/spawner blocks - chorus has no inventory/block-entity system yet, so a
/// placed chest/spawner simply starts out empty of that data, matching what pyrite's
/// own maintainer confirmed is fine for now.
pub(super) fn place_dungeon(generator: &OverworldGenerator, buffer: &mut OwnerBuffer, block_ids: &BlockIds, pos: IVec3, rand: &mut JavaRand) -> bool {
    let x_radius = rand.next_i32_bounded(2) + 2;
    let z_radius = rand.next_i32_bounded(2) + 2;
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
                            if wy == start.y && rand.next_i32_bounded(4) != 0 {
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
            let chest_pos = pos + IVec3::new(rand.next_i32_bounded(x_radius * 2 + 1) - x_radius, 0, rand.next_i32_bounded(z_radius * 2 + 1) - z_radius);
            
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
                        rand.next_i32_bounded(CHEST_SLOT_COUNT);
                    }
                }

                write(buffer, chest_pos.x, chest_pos.y, chest_pos.z, block_ids.chest);
                break;
            }
        }
    }

    let _entity_kind = rand.next_i32_bounded(4);
    write(buffer, pos.x, pos.y, pos.z, block_ids.mob_spawner);

    true
}
