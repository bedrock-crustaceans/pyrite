use glam::{DVec3, IVec3};

use crate::level::generator::java_rand::JavaRand;
use crate::level::generator::math::{MC_PI, mc_sin, mc_sin_cos};

use super::owner_buffer::{OwnerBuffer, read, write};
use super::{CHUNK_HEIGHT, CHUNK_WIDTH, OverworldGenerator};

pub(super) fn next_offset(rand: &mut JavaRand, max_y: i32, offset_xz: i32) -> IVec3 {
    IVec3::new(
        rand.next_i32_bounded(CHUNK_WIDTH as i32) + offset_xz,
        rand.next_i32_bounded(max_y),
        rand.next_i32_bounded(CHUNK_WIDTH as i32) + offset_xz,
    )
}

/// Places every ore/clay vein belonging to owner chunk `(owner_x, owner_z)` into its
/// own `OwnerBuffer`. Continues the same population RNG stream `rand` is already
/// partway through (see `population::populate_owner`).
pub fn populate_from(generator: &OverworldGenerator, buffer: &mut OwnerBuffer, owner_x: i32, owner_z: i32, rand: &mut JavaRand) {
    let block_ids = &generator.block_ids;
    let origin = IVec3::new(owner_x * CHUNK_WIDTH as i32, 0, owner_z * CHUNK_WIDTH as i32);

    // Clay veins only where the (unshifted) origin is underwater.
    for _ in 0..10 {
        let pos = origin + next_offset(rand, 128, 0);
        if read(generator, buffer, pos.x, pos.y, pos.z) == block_ids.water {
            place_vein(generator, buffer, block_ids.sand, block_ids.clay, 32, pos, rand);
        }
    }

    for _ in 0..20 {
        let pos = origin + next_offset(rand, 128, 0);
        place_vein(generator, buffer, block_ids.stone, block_ids.dirt, 32, pos, rand);
    }
    for _ in 0..10 {
        let pos = origin + next_offset(rand, 128, 0);
        place_vein(generator, buffer, block_ids.stone, block_ids.gravel, 32, pos, rand);
    }
    for _ in 0..20 {
        let pos = origin + next_offset(rand, 128, 0);
        place_vein(generator, buffer, block_ids.stone, block_ids.coal_ore, 16, pos, rand);
    }
    for _ in 0..20 {
        let pos = origin + next_offset(rand, 64, 0);
        place_vein(generator, buffer, block_ids.stone, block_ids.iron_ore, 8, pos, rand);
    }
    for _ in 0..2 {
        let pos = origin + next_offset(rand, 32, 0);
        place_vein(generator, buffer, block_ids.stone, block_ids.gold_ore, 8, pos, rand);
    }
    for _ in 0..8 {
        let pos = origin + next_offset(rand, 16, 0);
        place_vein(generator, buffer, block_ids.stone, block_ids.redstone_ore, 7, pos, rand);
    }
    for _ in 0..1 {
        let pos = origin + next_offset(rand, 16, 0);
        place_vein(generator, buffer, block_ids.stone, block_ids.diamond_ore, 7, pos, rand);
    }
    for _ in 0..1 {
        let pos = origin
            + IVec3::new(
                rand.next_i32_bounded(CHUNK_WIDTH as i32),
                rand.next_i32_bounded(16) + rand.next_i32_bounded(16),
                rand.next_i32_bounded(CHUNK_WIDTH as i32),
            );
        place_vein(generator, buffer, block_ids.stone, block_ids.lapis_ore, 6, pos, rand);
    }
}

/// Places one vein: a short line of blobby spheres running through `pos + 8` (in the
/// owner chunk's local space), replacing `replace_id` with `place_id`. Ported from the
/// reference's `VeinGenerator`.
fn place_vein(generator: &OverworldGenerator, buffer: &mut OwnerBuffer, replace_id: i32, place_id: i32, count: i32, pos: IVec3, rand: &mut JavaRand) {
    let angle = rand.next_float() * MC_PI;
    let (angle_sin, angle_cos) = mc_sin_cos(angle);
    let angle_sin = angle_sin * count as f32 / 8.0;
    let angle_cos = angle_cos * count as f32 / 8.0;

    let line_start = DVec3::new(
        ((pos.x + 8) as f32 + angle_sin) as f64,
        (pos.y + rand.next_i32_bounded(3) + 2) as f64,
        ((pos.z + 8) as f32 + angle_cos) as f64,
    );
    let line_stop = DVec3::new(
        ((pos.x + 8) as f32 - angle_sin) as f64,
        (pos.y + rand.next_i32_bounded(3) + 2) as f64,
        ((pos.z + 8) as f32 - angle_cos) as f64,
    );

    for i in 0..=count {
        let center = line_start + (line_stop - line_start) * i as f64 / count as f64;

        let base_size = rand.next_double() * count as f64 / 16.0;
        let size = ((mc_sin(i as f32 * MC_PI / count as f32) + 1.0) as f64) * base_size + 1.0;
        let half_size = size / 2.0;

        let start = (center - half_size).floor().as_ivec3();
        let stop = (center + half_size).floor().as_ivec3();

        for wx in start.x..=stop.x {
            for wz in start.z..=stop.z {
                for wy in start.y..=stop.y {
                    if !(0..CHUNK_HEIGHT as i32).contains(&wy) {
                        continue;
                    }

                    let delta = (DVec3::new(wx as f64, wy as f64, wz as f64) + 0.5 - center) / half_size;
                    if delta.length_squared() >= 1.0 {
                        continue;
                    }

                    if read(generator, buffer, wx, wy, wz) == replace_id {
                        write(buffer, wx, wy, wz, place_id);
                    }
                }
            }
        }
    }
}
