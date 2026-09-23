// Every loop index here also feeds into an offset/array-index calculation, not just
// `fill`'s own indexing, so clippy's iterator rewrite doesn't apply - see perlin.rs's
// identical situation/comment.
#![allow(clippy::needless_range_loop)]

use glam::{DVec3, IVec3};

use crate::level::generator::java_rand::JavaRand;

use super::owner_buffer::{OwnerBuffer, read, write};
use super::vein::next_offset;
use super::{BlockIds, CHUNK_WIDTH, OverworldGenerator};

/// Places every lake belonging to owner chunk `(owner_x, owner_z)` into its own
/// `OwnerBuffer`. Continues the same population RNG stream `rand` is already partway
/// through (see `population::populate_owner`) - water lakes, then lava lakes,
/// matching the reference's own order (both come before veins).
pub fn populate_from(generator: &OverworldGenerator, buffer: &mut OwnerBuffer, owner_x: i32, owner_z: i32, rand: &mut JavaRand) {
    let origin = IVec3::new(owner_x * CHUNK_WIDTH as i32, 0, owner_z * CHUNK_WIDTH as i32);
    let block_ids = &generator.block_ids;

    if rand.next_i32_bounded(4) == 0 {
        let pos = origin + next_offset(rand, 128, 8);
        place_lake(generator, buffer, block_ids, block_ids.water, pos, rand);
    }

    if rand.next_i32_bounded(8) == 0 {
        let pos = origin
            + IVec3::new(
                rand.next_i32_bounded(CHUNK_WIDTH as i32) + 8,
                {
                    let v = rand.next_i32_bounded(120);
                    rand.next_i32_bounded(v + 8)
                },
                rand.next_i32_bounded(CHUNK_WIDTH as i32) + 8,
            );

        if pos.y < 64 || rand.next_i32_bounded(10) == 0 {
            place_lake(generator, buffer, block_ids, block_ids.lava_still, pos, rand);
        }
    }
}

fn is_solid(block_ids: &BlockIds, id: i32) -> bool {
    id != block_ids.air && id != block_ids.water && id != block_ids.lava && id != block_ids.lava_still
}

fn is_fluid(block_ids: &BlockIds, id: i32) -> bool {
    id == block_ids.water || id == block_ids.lava || id == block_ids.lava_still
}

/// Places a lake of `fluid_id` (water or still lava) centered near `pos`, ported from
/// the reference's `LakeGenerator`.
#[allow(clippy::too_many_arguments)]
fn place_lake(generator: &OverworldGenerator, buffer: &mut OwnerBuffer, block_ids: &BlockIds, fluid_id: i32, mut pos: IVec3, rand: &mut JavaRand) -> bool {
    pos -= IVec3::new(8, 0, 8);

    while pos.y > 0 && read(generator, buffer, pos.x, pos.y, pos.z) == block_ids.air {
        pos.y -= 1;
    }
    pos.y -= 4;

    // [X][Z][Y], matching the reference's own indexing.
    let mut fill = [[[false; 8]; 16]; 16];

    let count = rand.next_i32_bounded(4) + 4;
    for _ in 0..count {
        let a = DVec3::new(rand.next_double(), rand.next_double(), rand.next_double()) * DVec3::new(6.0, 4.0, 6.0) + DVec3::new(3.0, 2.0, 3.0);
        let b = DVec3::new(rand.next_double(), rand.next_double(), rand.next_double()) * (DVec3::new(16.0, 8.0, 16.0) - a - DVec3::new(2.0, 4.0, 2.0)) + DVec3::new(1.0, 2.0, 1.0) + a / 2.0;
        let a = a / 2.0;

        for dx in 1..15usize {
            for dz in 1..15usize {
                for dy in 1..7usize {
                    let dist = (DVec3::new(dx as f64, dy as f64, dz as f64) - b) / a;
                    if dist.length_squared() < 1.0 {
                        fill[dx][dz][dy] = true;
                    }
                }
            }
        }
    }

    let is_edge = |fill: &[[[bool; 8]; 16]; 16], dx: usize, dz: usize, dy: usize| -> bool {
        !fill[dx][dz][dy]
            && ((dx < 15 && fill[dx + 1][dz][dy])
                || (dx > 0 && fill[dx - 1][dz][dy])
                || (dz < 15 && fill[dx][dz + 1][dy])
                || (dz > 0 && fill[dx][dz - 1][dy])
                || (dy < 7 && fill[dx][dz][dy + 1])
                || (dy > 0 && fill[dx][dz][dy - 1]))
    };

    for dx in 0..16usize {
        for dz in 0..16usize {
            for dy in 0..8usize {
                if is_edge(&fill, dx, dz, dy) {
                    let check_pos = pos + IVec3::new(dx as i32, dy as i32, dz as i32);
                    let check_id = read(generator, buffer, check_pos.x, check_pos.y, check_pos.z);
                    if (dy >= 4 && is_fluid(block_ids, check_id)) || (dy < 4 && !is_solid(block_ids, check_id) && check_id != fluid_id) {
                        return false;
                    }
                }
            }
        }
    }

    for dx in 0..16usize {
        for dz in 0..16usize {
            for dy in 0..8usize {
                if fill[dx][dz][dy] {
                    let place_pos = pos + IVec3::new(dx as i32, dy as i32, dz as i32);
                    let id = if dy >= 4 { block_ids.air } else { fluid_id };
                    write(buffer, place_pos.x, place_pos.y, place_pos.z, id);
                }
            }
        }
    }

    // The reference only converts dirt at the lake's rim to grass where it also sees
    // sky light - chorus has no lighting engine yet to ask, so this always converts,
    // which is right for the common case (a rim exposed to open air) and only wrong
    // for the rarer case of a lake basin covered by an overhang. Cosmetic only: no
    // RNG is involved here, so it can't affect anything placed after this feature.
    for dx in 0..16usize {
        for dz in 0..16usize {
            for dy in 4..8usize {
                if fill[dx][dz][dy] {
                    let below = pos + IVec3::new(dx as i32, dy as i32 - 1, dz as i32);
                    if read(generator, buffer, below.x, below.y, below.z) == block_ids.dirt {
                        write(buffer, below.x, below.y, below.z, block_ids.grass);
                    }
                }
            }
        }
    }

    if fluid_id == block_ids.lava_still {
        for dx in 0..16usize {
            for dz in 0..16usize {
                for dy in 0..8usize {
                    if is_edge(&fill, dx, dz, dy) && (dy < 4 || rand.next_i32_bounded(2) != 0) {
                        let place_pos = pos + IVec3::new(dx as i32, dy as i32, dz as i32);
                        let id = read(generator, buffer, place_pos.x, place_pos.y, place_pos.z);
                        if is_solid(block_ids, id) {
                            write(buffer, place_pos.x, place_pos.y, place_pos.z, block_ids.stone);
                        }
                    }
                }
            }
        }
    }

    true
}
