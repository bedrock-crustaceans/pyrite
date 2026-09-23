use glam::IVec3;

use crate::level::generator::java_rand::JavaRand;

use super::biome::Biome;
use super::owner_buffer::{OwnerBuffer, read, write};
use super::tree::is_leaves;
use super::vein::next_offset;
use super::{BlockIds, OverworldGenerator};

/// Matches the reference's `Face::HORIZONTAL` order exactly - sugar cane placement
/// rolls fresh RNG values per adjacent-water face it finds, so the iteration order
/// affects the resulting RNG stream, not just which faces get checked.
const HORIZONTAL_FACES: [IVec3; 4] = [IVec3::new(0, 0, -1), IVec3::new(0, 0, 1), IVec3::new(-1, 0, 0), IVec3::new(1, 0, 0)];

/// Places every plant belonging to owner chunk `(owner_x, owner_z)` into its own
/// `OwnerBuffer`. Ported from the reference's dandelion/tall-grass/dead-bush/poppy/
/// mushroom/sugar-cane/pumpkin/cactus population calls, in their relative order
/// (continuing the same stream `rand` is already partway through - see
/// `population::populate_owner`).
pub fn populate_from(generator: &OverworldGenerator, buffer: &mut OwnerBuffer, owner_x: i32, owner_z: i32, rand: &mut JavaRand) {
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
        place_flower_patch(generator, buffer, pos, block_ids.dandelion, 64, false, rand);
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
        if biome == Biome::RainForest && rand.next_i32_bounded(3) != 0 {
            plant_id = block_ids.fern;
        }
        let pos = origin + next_offset(rand, 128, 8);
        place_flower_patch(generator, buffer, pos, plant_id, 128, true, rand);
    }

    if biome == Biome::Desert {
        for _ in 0..2 {
            let pos = origin + next_offset(rand, 128, 8);
            place_flower_patch(generator, buffer, pos, block_ids.deadbush, 4, true, rand);
        }
    }

    if rand.next_i32_bounded(2) == 0 {
        let pos = origin + next_offset(rand, 128, 8);
        place_flower_patch(generator, buffer, pos, block_ids.poppy, 64, false, rand);
    }

    if rand.next_i32_bounded(4) == 0 {
        let pos = origin + next_offset(rand, 128, 8);
        place_flower_patch(generator, buffer, pos, block_ids.brown_mushroom, 64, false, rand);
    }

    if rand.next_i32_bounded(8) == 0 {
        let pos = origin + next_offset(rand, 128, 8);
        place_flower_patch(generator, buffer, pos, block_ids.red_mushroom, 64, false, rand);
    }

    for _ in 0..10 {
        let pos = origin + next_offset(rand, 128, 8);
        place_sugar_canes(generator, buffer, pos, rand);
    }

    if rand.next_i32_bounded(32) == 0 {
        let pos = origin + next_offset(rand, 128, 8);
        place_pumpkin(generator, buffer, pos, rand);
    }

    if biome == Biome::Desert {
        for _ in 0..10 {
            let pos = origin + next_offset(rand, 128, 8);
            place_cactus(generator, buffer, pos, rand);
        }
    }
}

/// A patch of `count` single-block plants scattered around `pos` (dandelion, poppy,
/// tall grass/fern, dead bush, both mushroom kinds). `find_ground` walks `pos` down
/// through air/leaves first, matching the reference's `SimpleTreeGenerator`-adjacent
/// ground search used by tall grass and dead bush.
fn place_flower_patch(generator: &OverworldGenerator, buffer: &mut OwnerBuffer, mut pos: IVec3, plant_id: i32, count: i32, find_ground: bool, rand: &mut JavaRand) {
    let block_ids = &generator.block_ids;

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
                rand.next_i32_bounded(8) - rand.next_i32_bounded(8),
                rand.next_i32_bounded(4) - rand.next_i32_bounded(4),
                rand.next_i32_bounded(8) - rand.next_i32_bounded(8),
            );

        if read(generator, buffer, place_pos.x, place_pos.y, place_pos.z) != block_ids.air {
            continue;
        }
        if !can_place_flower(generator, buffer, block_ids, place_pos, plant_id) {
            continue;
        }

        write(buffer, place_pos.x, place_pos.y, place_pos.z, plant_id);
    }
}

fn can_place_flower(generator: &OverworldGenerator, buffer: &OwnerBuffer, block_ids: &BlockIds, pos: IVec3, plant_id: i32) -> bool {
    let below = read(generator, buffer, pos.x, pos.y - 1, pos.z);
    if plant_id == block_ids.deadbush {
        below == block_ids.sand
    } else if plant_id == block_ids.red_mushroom || plant_id == block_ids.brown_mushroom {
        below != block_ids.air && below != block_ids.water
    } else {
        below == block_ids.grass || below == block_ids.dirt
    }
}

/// Sugar cane: for each horizontal neighbor of `pos`'s ground that touches water, rolls
/// a height and (redundantly, matching the reference's own `can_place_sugar_canes`,
/// which re-scans every horizontal neighbor rather than just the triggering one) places
/// a column of that height if the ground is grass/dirt. Faithfully reproduces the
/// reference's quirk of re-rolling and re-placing once per adjacent water face, not
/// just once per `pos`.
fn place_sugar_canes(generator: &OverworldGenerator, buffer: &mut OwnerBuffer, pos: IVec3, rand: &mut JavaRand) {
    let block_ids = &generator.block_ids;

    for _ in 0..20 {
        let place_pos = pos + IVec3::new(rand.next_i32_bounded(4) - rand.next_i32_bounded(4), 0, rand.next_i32_bounded(4) - rand.next_i32_bounded(4));

        if read(generator, buffer, place_pos.x, place_pos.y, place_pos.z) != block_ids.air {
            continue;
        }

        let below_pos = place_pos - IVec3::Y;

        for face in HORIZONTAL_FACES {
            let water_pos = below_pos + face;
            if read(generator, buffer, water_pos.x, water_pos.y, water_pos.z) != block_ids.water {
                continue;
            }

            let v = rand.next_i32_bounded(3) + 1;
            let height = rand.next_i32_bounded(v) + 2;

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

fn place_pumpkin(generator: &OverworldGenerator, buffer: &mut OwnerBuffer, pos: IVec3, rand: &mut JavaRand) {
    let block_ids = &generator.block_ids;

    for _ in 0..64 {
        let place_pos = pos
            + IVec3::new(
                rand.next_i32_bounded(8) - rand.next_i32_bounded(8),
                rand.next_i32_bounded(4) - rand.next_i32_bounded(4),
                rand.next_i32_bounded(8) - rand.next_i32_bounded(8),
            );

        if read(generator, buffer, place_pos.x, place_pos.y, place_pos.z) != block_ids.air {
            continue;
        }
        let below = read(generator, buffer, place_pos.x, place_pos.y - 1, place_pos.z);
        if below != block_ids.grass {
            continue;
        }

        // Facing isn't modeled (no permutation-state writer exists yet for any block
        // in this generator - see oak_log's fixed default axis), but the roll is still
        // consumed here to keep the RNG stream in sync with the reference.
        let _facing = rand.next_i32_bounded(4);
        write(buffer, place_pos.x, place_pos.y, place_pos.z, block_ids.pumpkin);
    }
}

fn place_cactus(generator: &OverworldGenerator, buffer: &mut OwnerBuffer, pos: IVec3, rand: &mut JavaRand) {
    let block_ids = &generator.block_ids;

    for _ in 0..10 {
        let place_pos = pos
            + IVec3::new(
                rand.next_i32_bounded(8) - rand.next_i32_bounded(8),
                rand.next_i32_bounded(4) - rand.next_i32_bounded(4),
                rand.next_i32_bounded(8) - rand.next_i32_bounded(8),
            );

        if read(generator, buffer, place_pos.x, place_pos.y, place_pos.z) != block_ids.air {
            continue;
        }

        let v = rand.next_i32_bounded(3) + 1;
        let height = rand.next_i32_bounded(v) + 1;

        for dy in 0..height {
            if can_place_cactus(generator, buffer, block_ids, place_pos) {
                write(buffer, place_pos.x, place_pos.y + dy, place_pos.z, block_ids.cactus);
            }
        }
    }
}

fn can_place_cactus(generator: &OverworldGenerator, buffer: &OwnerBuffer, block_ids: &BlockIds, pos: IVec3) -> bool {
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
