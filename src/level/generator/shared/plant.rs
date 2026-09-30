use glam::IVec3;

use super::block_ids::BlockIds;
use super::material::{can_see_sky, is_opaque_cube, is_solid, is_water, sky_light};
use super::quad_chunk_buffer::{QuadChunkBuffer, read, write};
use super::tree::is_leaves;
use super::{CHUNK_HEIGHT, HORIZONTAL_FACES};
use crate::rand::java::JavaRand;
use crate::rand::primitives::Bound;

#[derive(Clone, Copy)]
pub enum PlantRule {
    Flower,
    DeadBush,
    Mushroom,
}

impl PlantRule {
    fn can_stay(self, buffer: &QuadChunkBuffer, block_ids: &BlockIds, pos: IVec3) -> bool {
        let below = read(buffer, pos.x, pos.y - 1, pos.z);
        match self {
            PlantRule::Flower => (below == block_ids.grass || below == block_ids.dirt) && is_lit(buffer, block_ids, pos),
            PlantRule::DeadBush => below == block_ids.sand && is_lit(buffer, block_ids, pos),
            PlantRule::Mushroom => (0..CHUNK_HEIGHT as i32).contains(&pos.y) && is_opaque_cube(block_ids, below) && sky_light(buffer, block_ids, pos.x, pos.y, pos.z) < 13,
        }
    }
}

fn is_lit(buffer: &QuadChunkBuffer, block_ids: &BlockIds, pos: IVec3) -> bool {
    sky_light(buffer, block_ids, pos.x, pos.y, pos.z) >= 8 || can_see_sky(buffer, block_ids, pos.x, pos.y, pos.z)
}

fn scatter(rand: &mut JavaRand, pos: IVec3) -> IVec3 {
    pos + IVec3::new(
        rand.random_with::<i32>(Bound::new(8)) - rand.random_with::<i32>(Bound::new(8)),
        rand.random_with::<i32>(Bound::new(4)) - rand.random_with::<i32>(Bound::new(4)),
        rand.random_with::<i32>(Bound::new(8)) - rand.random_with::<i32>(Bound::new(8)),
    )
}

/// `WorldGenFlowers`, and with `find_ground` set, `WorldGenTallGrass`/`WorldGenDeadBush`.
#[allow(clippy::too_many_arguments)]
pub fn place_plants(buffer: &mut QuadChunkBuffer, mut pos: IVec3, block_ids: &BlockIds, plant_id: i32, count: i32, find_ground: bool, rule: PlantRule, rand: &mut JavaRand) {
    if find_ground {
        while pos.y > 0 {
            let id = read(buffer, pos.x, pos.y, pos.z);
            if id != block_ids.air && !is_leaves(block_ids, id) {
                break;
            }
            pos.y -= 1;
        }
    }

    for _ in 0..count {
        let place_pos = scatter(rand, pos);
        if read(buffer, place_pos.x, place_pos.y, place_pos.z) == block_ids.air && rule.can_stay(buffer, block_ids, place_pos) {
            write(buffer, place_pos.x, place_pos.y, place_pos.z, plant_id);
        }
    }
}

fn water_beside(buffer: &QuadChunkBuffer, block_ids: &BlockIds, pos: IVec3) -> bool {
    HORIZONTAL_FACES.iter().any(|&face| {
        let side = pos + face;
        is_water(block_ids, read(buffer, side.x, side.y, side.z))
    })
}

fn can_sugar_cane_stay(buffer: &QuadChunkBuffer, block_ids: &BlockIds, pos: IVec3) -> bool {
    let below_pos = pos - IVec3::Y;
    let below = read(buffer, below_pos.x, below_pos.y, below_pos.z);
    below == block_ids.reeds || ((below == block_ids.grass || below == block_ids.dirt) && water_beside(buffer, block_ids, below_pos))
}

pub fn place_sugar_canes(buffer: &mut QuadChunkBuffer, pos: IVec3, block_ids: &BlockIds, rand: &mut JavaRand) {
    for _ in 0..20 {
        let place_pos = pos
            + IVec3::new(
                rand.random_with::<i32>(Bound::new(4)) - rand.random_with::<i32>(Bound::new(4)),
                0,
                rand.random_with::<i32>(Bound::new(4)) - rand.random_with::<i32>(Bound::new(4)),
            );

        if read(buffer, place_pos.x, place_pos.y, place_pos.z) != block_ids.air || !water_beside(buffer, block_ids, place_pos - IVec3::Y) {
            continue;
        }

        let bound = rand.random_with::<i32>(Bound::new(3)) + 1;
        let height = 2 + rand.random_with::<i32>(Bound::new(bound));
        for dy in 0..height {
            let cane_pos = place_pos + IVec3::new(0, dy, 0);
            if can_sugar_cane_stay(buffer, block_ids, cane_pos) {
                write(buffer, cane_pos.x, cane_pos.y, cane_pos.z, block_ids.reeds);
            }
        }
    }
}

pub fn place_pumpkin(buffer: &mut QuadChunkBuffer, pos: IVec3, block_ids: &BlockIds, rand: &mut JavaRand) {
    for _ in 0..64 {
        let place_pos = scatter(rand, pos);
        if read(buffer, place_pos.x, place_pos.y, place_pos.z) == block_ids.air && read(buffer, place_pos.x, place_pos.y - 1, place_pos.z) == block_ids.grass {
            let facing = rand.random_with::<i32>(Bound::new(4));
            write(buffer, place_pos.x, place_pos.y, place_pos.z, block_ids.pumpkin_facings[facing as usize]);
        }
    }
}

fn can_cactus_stay(buffer: &QuadChunkBuffer, block_ids: &BlockIds, pos: IVec3) -> bool {
    let open_sides = HORIZONTAL_FACES.iter().all(|&face| {
        let side = pos + face;
        !is_solid(block_ids, read(buffer, side.x, side.y, side.z))
    });
    let below = read(buffer, pos.x, pos.y - 1, pos.z);
    open_sides && (below == block_ids.cactus || below == block_ids.sand)
}

pub fn place_cactus(buffer: &mut QuadChunkBuffer, pos: IVec3, block_ids: &BlockIds, rand: &mut JavaRand) {
    for _ in 0..10 {
        let place_pos = scatter(rand, pos);
        if read(buffer, place_pos.x, place_pos.y, place_pos.z) != block_ids.air {
            continue;
        }

        let bound = rand.random_with::<i32>(Bound::new(3)) + 1;
        let height = 1 + rand.random_with::<i32>(Bound::new(bound));
        for dy in 0..height {
            let cactus_pos = place_pos + IVec3::new(0, dy, 0);
            if can_cactus_stay(buffer, block_ids, cactus_pos) {
                write(buffer, cactus_pos.x, cactus_pos.y, cactus_pos.z, block_ids.cactus);
            }
        }
    }
}
