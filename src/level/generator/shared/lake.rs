#![allow(clippy::needless_range_loop)]

use glam::{DVec3, IVec3};

use super::CHUNK_WIDTH;
use super::block_ids::BlockIds;
use super::material::{has_sky_light, is_liquid, is_solid};
use super::quad_chunk_buffer::{QuadChunkBuffer, column_at, read};
use super::vein::next_offset;
use crate::rand::java::JavaRand;
use crate::rand::primitives::Bound;

pub fn populate_from(buffer: &mut QuadChunkBuffer, owner_x: i32, owner_z: i32, block_ids: &BlockIds, rand: &mut JavaRand) {
    let origin = IVec3::new(owner_x * CHUNK_WIDTH as i32, 0, owner_z * CHUNK_WIDTH as i32);

    if rand.random_with::<i32>(Bound::new(4)) == 0 {
        let pos = origin + next_offset(rand, 128, 8);
        place_lake(buffer, block_ids, block_ids.water, pos, rand);
    }

    if rand.random_with::<i32>(Bound::new(8)) == 0 {
        let bound = CHUNK_WIDTH as i32;
        let bound1 = CHUNK_WIDTH as i32;
        let pos = origin
            + IVec3::new(
                rand.random_with::<i32>(Bound::new(bound1)) + 8,
                {
                    let v = rand.random_with::<i32>(Bound::new(120));
                    let bound1 = v + 8;
                    rand.random_with::<i32>(Bound::new(bound1))
                },
                rand.random_with::<i32>(Bound::new(bound)) + 8,
            );

        if pos.y < 64 || rand.random_with::<i32>(Bound::new(10)) == 0 {
            place_lake(buffer, block_ids, block_ids.lava_still, pos, rand);
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn place_lake(buffer: &mut QuadChunkBuffer, block_ids: &BlockIds, fluid_id: i32, mut pos: IVec3, rand: &mut JavaRand) -> bool {
    pos -= IVec3::new(8, 0, 8);

    while pos.y > 0 && read(buffer, pos.x, pos.y, pos.z) == block_ids.air {
        pos.y -= 1;
    }
    pos.y -= 4;

    let mut fill = [[[false; 8]; 16]; 16];

    let count = rand.random_with::<i32>(Bound::new(4)) + 4;
    for _ in 0..count {
        let a = DVec3::new(rand.random::<f64>(), rand.random::<f64>(), rand.random::<f64>()) * DVec3::new(6.0, 4.0, 6.0) + DVec3::new(3.0, 2.0, 3.0);
        let b = DVec3::new(rand.random::<f64>(), rand.random::<f64>(), rand.random::<f64>()) * (DVec3::new(16.0, 8.0, 16.0) - a - DVec3::new(2.0, 4.0, 2.0)) + DVec3::new(1.0, 2.0, 1.0) + a / 2.0;
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
            let column = column_at(pos.x + dx as i32, pos.z + dz as i32);
            for dy in 0..8usize {
                if is_edge(&fill, dx, dz, dy) {
                    let check_id = column.read(buffer, pos.y + dy as i32);
                    if (dy >= 4 && is_liquid(block_ids, check_id)) || (dy < 4 && !is_solid(block_ids, check_id) && check_id != fluid_id) {
                        return false;
                    }
                }
            }
        }
    }

    for dx in 0..16usize {
        for dz in 0..16usize {
            let column = column_at(pos.x + dx as i32, pos.z + dz as i32);
            for dy in 0..8usize {
                if fill[dx][dz][dy] {
                    let id = if dy >= 4 { block_ids.air } else { fluid_id };
                    column.write(buffer, pos.y + dy as i32, id);
                }
            }
        }
    }

    for dx in 0..16usize {
        for dz in 0..16usize {
            let column = column_at(pos.x + dx as i32, pos.z + dz as i32);
            for dy in 4..8usize {
                if fill[dx][dz][dy] {
                    let below_y = pos.y + dy as i32 - 1;
                    if column.read(buffer, below_y) == block_ids.dirt && has_sky_light(buffer, block_ids, pos.x + dx as i32, below_y + 1, pos.z + dz as i32) {
                        column.write(buffer, below_y, block_ids.grass);
                    }
                }
            }
        }
    }

    if fluid_id == block_ids.lava_still {
        for dx in 0..16usize {
            for dz in 0..16usize {
                let column = column_at(pos.x + dx as i32, pos.z + dz as i32);
                for dy in 0..8usize {
                    if is_edge(&fill, dx, dz, dy) && (dy < 4 || rand.random_with::<i32>(Bound::new(2)) != 0) {
                        let id = column.read(buffer, pos.y + dy as i32);
                        if is_solid(block_ids, id) {
                            column.write(buffer, pos.y + dy as i32, block_ids.stone);
                        }
                    }
                }
            }
        }
    }

    true
}
