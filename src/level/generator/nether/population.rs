use glam::IVec3;

use super::{BlockIds, CHUNK_HEIGHT, CHUNK_WIDTH, NetherGenerator, chunk_seed};
use crate::level::generator::shared::phases::Population;
use crate::level::generator::shared::quad_chunk_buffer::{QuadChunkBuffer, read, write};
use crate::rand::java::JavaRand;
use crate::rand::primitives::Bound;

const FACE_NEIGHBORS: [IVec3; 6] = [
    IVec3::new(-1, 0, 0),
    IVec3::new(1, 0, 0),
    IVec3::new(0, -1, 0),
    IVec3::new(0, 1, 0),
    IVec3::new(0, 0, -1),
    IVec3::new(0, 0, 1),
];

impl Population for NetherGenerator {
    fn run_population_raw(&self, owner_x: i32, owner_z: i32, buffer: &mut QuadChunkBuffer) {
        let mut rand = JavaRand::new(chunk_seed(self.seed, owner_x, owner_z));
        let origin = IVec3::new(owner_x * CHUNK_WIDTH as i32, 0, owner_z * CHUNK_WIDTH as i32);
        let block_ids = &self.block_ids;

        for _ in 0..8 {
            let pos = next_mid_height_position(&mut rand, origin);
            place_lava_spring(self, buffer, block_ids, pos);
        }

        let fire_bound = rand.random_with::<i32>(Bound::new(10)) + 1;
        let fire_count = rand.random_with::<i32>(Bound::new(fire_bound)) + 1;
        for _ in 0..fire_count {
            let pos = next_mid_height_position(&mut rand, origin);
            place_fire(self, buffer, block_ids, pos, &mut rand);
        }

        let glowstone_bound = rand.random_with::<i32>(Bound::new(10)) + 1;
        let glowstone_cluster_count = rand.random_with::<i32>(Bound::new(glowstone_bound));
        for _ in 0..glowstone_cluster_count {
            let pos = next_mid_height_position(&mut rand, origin);
            place_glowstone_cluster(self, buffer, block_ids, pos, &mut rand);
        }

        for _ in 0..10 {
            let pos = next_full_height_position(&mut rand, origin);
            place_glowstone_cluster(self, buffer, block_ids, pos, &mut rand);
        }

        // `next_i32_bounded(1)` is always 0 - the reference's own check is a no-op that
        // still consumes a draw, so every chunk gets exactly one attempt at each mushroom.
        if rand.random_with::<i32>(Bound::new(1)) == 0 {
            let pos = next_full_height_position(&mut rand, origin);
            place_mushroom(self, buffer, block_ids, block_ids.brown_mushroom, pos, &mut rand);
        }
        if rand.random_with::<i32>(Bound::new(1)) == 0 {
            let pos = next_full_height_position(&mut rand, origin);
            place_mushroom(self, buffer, block_ids, block_ids.red_mushroom, pos, &mut rand);
        }
    }
}

fn next_mid_height_position(rand: &mut JavaRand, origin: IVec3) -> IVec3 {
    let bound = CHUNK_WIDTH as i32;
    let x = rand.random_with::<i32>(Bound::new(bound)) + 8;
    let y = rand.random_with::<i32>(Bound::new(120)) + 4;
    let bound = CHUNK_WIDTH as i32;
    let z = rand.random_with::<i32>(Bound::new(bound)) + 8;
    origin + IVec3::new(x, y, z)
}

fn next_full_height_position(rand: &mut JavaRand, origin: IVec3) -> IVec3 {
    let bound = CHUNK_WIDTH as i32;
    let x = rand.random_with::<i32>(Bound::new(bound)) + 8;
    let bound = CHUNK_HEIGHT as i32;
    let y = rand.random_with::<i32>(Bound::new(bound));
    let bound = CHUNK_WIDTH as i32;
    let z = rand.random_with::<i32>(Bound::new(bound)) + 8;
    origin + IVec3::new(x, y, z)
}

fn place_lava_spring(generator: &NetherGenerator, buffer: &mut QuadChunkBuffer, block_ids: &BlockIds, pos: IVec3) {
    if read(generator, buffer, pos.x, pos.y + 1, pos.z) != block_ids.netherrack {
        return;
    }

    let here = read(generator, buffer, pos.x, pos.y, pos.z);
    if here != block_ids.air && here != block_ids.netherrack {
        return;
    }

    let gate_neighbors = [IVec3::new(-1, 0, 0), IVec3::new(1, 0, 0), IVec3::new(0, 0, -1), IVec3::new(0, 0, 1), IVec3::new(0, -1, 0)];

    let mut netherrack_count = 0;
    let mut air_count = 0;
    for offset in gate_neighbors {
        let neighbor = pos + offset;
        let id = read(generator, buffer, neighbor.x, neighbor.y, neighbor.z);
        if id == block_ids.netherrack {
            netherrack_count += 1;
        }
        if id == block_ids.air {
            air_count += 1;
        }
    }

    if netherrack_count == 4 && air_count == 1 {
        write(buffer, pos.x, pos.y, pos.z, block_ids.lava);
    }
}

fn place_fire(generator: &NetherGenerator, buffer: &mut QuadChunkBuffer, block_ids: &BlockIds, pos: IVec3, rand: &mut JavaRand) {
    for _ in 0..64 {
        let target = pos
            + IVec3::new(
                rand.random_with::<i32>(Bound::new(8)) - rand.random_with::<i32>(Bound::new(8)),
                rand.random_with::<i32>(Bound::new(4)) - rand.random_with::<i32>(Bound::new(4)),
                rand.random_with::<i32>(Bound::new(8)) - rand.random_with::<i32>(Bound::new(8)),
            );

        if read(generator, buffer, target.x, target.y, target.z) == block_ids.air && read(generator, buffer, target.x, target.y - 1, target.z) == block_ids.netherrack {
            write(buffer, target.x, target.y, target.z, block_ids.fire);
        }
    }
}

fn place_glowstone_cluster(generator: &NetherGenerator, buffer: &mut QuadChunkBuffer, block_ids: &BlockIds, pos: IVec3, rand: &mut JavaRand) {
    if read(generator, buffer, pos.x, pos.y, pos.z) != block_ids.air {
        return;
    }
    if read(generator, buffer, pos.x, pos.y + 1, pos.z) != block_ids.netherrack {
        return;
    }

    write(buffer, pos.x, pos.y, pos.z, block_ids.glowstone);

    for _ in 0..1500 {
        let target = pos
            + IVec3::new(
                rand.random_with::<i32>(Bound::new(8)) - rand.random_with::<i32>(Bound::new(8)),
                -rand.random_with::<i32>(Bound::new(12)),
                rand.random_with::<i32>(Bound::new(8)) - rand.random_with::<i32>(Bound::new(8)),
            );

        if read(generator, buffer, target.x, target.y, target.z) != block_ids.air {
            continue;
        }

        let mut glowstone_neighbors = 0;
        for offset in FACE_NEIGHBORS {
            let neighbor = target + offset;
            if read(generator, buffer, neighbor.x, neighbor.y, neighbor.z) == block_ids.glowstone {
                glowstone_neighbors += 1;
            }
        }

        if glowstone_neighbors == 1 {
            write(buffer, target.x, target.y, target.z, block_ids.glowstone);
        }
    }
}

fn place_mushroom(generator: &NetherGenerator, buffer: &mut QuadChunkBuffer, block_ids: &BlockIds, mushroom_id: i32, pos: IVec3, rand: &mut JavaRand) {
    for _ in 0..64 {
        let target = pos
            + IVec3::new(
                rand.random_with::<i32>(Bound::new(8)) - rand.random_with::<i32>(Bound::new(8)),
                rand.random_with::<i32>(Bound::new(4)) - rand.random_with::<i32>(Bound::new(4)),
                rand.random_with::<i32>(Bound::new(8)) - rand.random_with::<i32>(Bound::new(8)),
            );

        if read(generator, buffer, target.x, target.y, target.z) == block_ids.air && can_place_mushroom(generator, buffer, block_ids, target) {
            write(buffer, target.x, target.y, target.z, mushroom_id);
        }
    }
}

fn can_place_mushroom(generator: &NetherGenerator, buffer: &QuadChunkBuffer, block_ids: &BlockIds, pos: IVec3) -> bool {
    if !(0..CHUNK_HEIGHT as i32).contains(&pos.y) {
        return false;
    }
    let below = read(generator, buffer, pos.x, pos.y - 1, pos.z);
    below == block_ids.netherrack || below == block_ids.soul_sand || below == block_ids.gravel || below == block_ids.bedrock || below == block_ids.glowstone
}
