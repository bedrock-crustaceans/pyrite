use glam::IVec3;

use crate::level::generator::java_rand::JavaRand;
use crate::level::generator::math::MC_PI;
use super::biome::Biome;
use super::owner_buffer::{OwnerBuffer, read, write};
use super::{BlockIds, CHUNK_HEIGHT, CHUNK_WIDTH, OverworldGenerator};

/// Places every tree belonging to owner chunk `(owner_x, owner_z)` into its own
/// `OwnerBuffer`. Continues the same population RNG stream `rand` is already partway
/// through (see `population::populate_owner`).
pub fn populate_from(generator: &OverworldGenerator, buffer: &mut OwnerBuffer, owner_x: i32, owner_z: i32, rand: &mut JavaRand) {
    let origin = IVec3::new(owner_x * CHUNK_WIDTH as i32, 0, owner_z * CHUNK_WIDTH as i32);
    let biome = generator.biome_at(origin.x + 16, origin.z + 16);

    let feature_value = generator.feature_noise_at(origin.x as f64 * 0.5, origin.z as f64 * 0.5);
    let base_tree_count = ((feature_value / 8.0 + rand.next_double() * 4.0 + 4.0) / 3.0) as i32;

    let mut tree_count = 0;
    if rand.next_i32_bounded(10) == 0 {
        tree_count += 1;
    }

    match biome {
        Biome::Taiga | Biome::RainForest | Biome::Forest => tree_count += base_tree_count + 5,
        Biome::SeasonalForest => tree_count += base_tree_count + 2,
        Biome::Desert | Biome::Tundra | Biome::Plains => tree_count -= 20,
        _ => {}
    }

    if tree_count <= 0 {
        return;
    }

    let block_ids = &generator.block_ids;

    for _ in 0..tree_count {
        let tree_x = origin.x + rand.next_i32_bounded(CHUNK_WIDTH as i32) + 8;
        let tree_z = origin.z + rand.next_i32_bounded(CHUNK_WIDTH as i32) + 8;
        let tree_y = surface_height(generator, buffer, tree_x, tree_z);
        let pos = IVec3::new(tree_x, tree_y, tree_z);

        match biome {
            Biome::Taiga => {
                if rand.next_i32_bounded(3) == 0 {
                    place_spruce1_tree(generator, buffer, pos, rand);
                } else {
                    place_spruce2_tree(generator, buffer, pos, rand);
                }
            }
            Biome::Forest => {
                if rand.next_i32_bounded(5) == 0 {
                    place_simple_tree(generator, buffer, pos, 5, block_ids.birch_log, block_ids.birch_leaves, rand);
                } else if rand.next_i32_bounded(3) == 0 {
                    place_big_tree(generator, buffer, pos, rand);
                } else {
                    place_simple_tree(generator, buffer, pos, 4, block_ids.oak_log, block_ids.oak_leaves, rand);
                }
            }
            Biome::RainForest => {
                if rand.next_i32_bounded(3) == 0 {
                    place_big_tree(generator, buffer, pos, rand);
                } else {
                    place_simple_tree(generator, buffer, pos, 4, block_ids.oak_log, block_ids.oak_leaves, rand);
                }
            }
            _ => {
                if rand.next_i32_bounded(10) == 0 {
                    place_big_tree(generator, buffer, pos, rand);
                } else {
                    place_simple_tree(generator, buffer, pos, 4, block_ids.oak_log, block_ids.oak_leaves, rand);
                }
            }
        }
    }
}

/// The reference's `world.get_height`: the first non-air block's Y, plus one, scanning
/// down from the build limit.
fn surface_height(generator: &OverworldGenerator, buffer: &OwnerBuffer, wx: i32, wz: i32) -> i32 {
    let air = generator.block_ids.air;
    for wy in (0..CHUNK_HEIGHT as i32).rev() {
        if read(generator, buffer, wx, wy, wz) != air {
            return wy + 1;
        }
    }
    0
}

pub(super) fn is_leaves(block_ids: &BlockIds, id: i32) -> bool {
    id == block_ids.oak_leaves || id == block_ids.birch_leaves || id == block_ids.spruce_leaves
}

/// Checks that a tree can grow: the ground below is grass/dirt, and there's clear
/// (air or leaves) space in a `check_radius(y)`-wide column above it.
fn check_tree(generator: &OverworldGenerator, buffer: &OwnerBuffer, pos: IVec3, height: i32, check_radius: impl Fn(i32) -> i32) -> bool {
    let max_y = pos.y + height + 1;
    if pos.y < 1 || max_y >= CHUNK_HEIGHT as i32 {
        return false;
    }

    let block_ids = &generator.block_ids;
    let below = read(generator, buffer, pos.x, pos.y - 1, pos.z);
    if below != block_ids.grass && below != block_ids.dirt {
        return false;
    }

    for wy in pos.y..=max_y {
        let radius = check_radius(wy);
        for wx in pos.x - radius..=pos.x + radius {
            for wz in pos.z - radius..=pos.z + radius {
                let id = read(generator, buffer, wx, wy, wz);
                if id == block_ids.air || is_leaves(block_ids, id) {
                    continue;
                }
                return false;
            }
        }
    }

    true
}

/// A plain single-trunk tree (oak, birch): a short trunk topped with a few layers of
/// leaves that taper inward. Ported from the reference's `SimpleTreeGenerator`.
fn place_simple_tree(generator: &OverworldGenerator, buffer: &mut OwnerBuffer, pos: IVec3, min_height: i32, log_id: i32, leaves_id: i32, rand: &mut JavaRand) -> bool {
    let height = rand.next_i32_bounded(3) + min_height;

    let check_radius = |y: i32| {
        if y == pos.y {
            0
        } else if y >= pos.y + height - 1 {
            2
        } else {
            1
        }
    };

    if !check_tree(generator, buffer, pos, height, check_radius) {
        return false;
    }

    let block_ids = &generator.block_ids;
    write(buffer, pos.x, pos.y - 1, pos.z, block_ids.dirt);

    for wy in (pos.y + height - 3)..=(pos.y + height) {
        let dy = wy - (pos.y + height);
        let radius = 1 - dy / 2;

        for wx in pos.x - radius..=pos.x + radius {
            for wz in pos.z - radius..=pos.z + radius {
                let dx = (wx - pos.x).abs();
                let dz = (wz - pos.z).abs();
                if dx != radius || dz != radius || (rand.next_i32_bounded(2) != 0 && dy != 0) {
                    let id = read(generator, buffer, wx, wy, wz);
                    if id == block_ids.air || is_leaves(block_ids, id) {
                        write(buffer, wx, wy, wz, leaves_id);
                    }
                }
            }
        }
    }

    for wy in pos.y..(pos.y + height) {
        let id = read(generator, buffer, pos.x, wy, pos.z);
        if id == block_ids.air || is_leaves(block_ids, id) {
            write(buffer, pos.x, wy, pos.z, log_id);
        }
    }

    true
}

/// A spruce tree that tapers to a point (variant 1): leaves start partway up the
/// trunk and shrink back to nothing near the top. Ported from `Spruce1TreeGenerator`.
fn place_spruce1_tree(generator: &OverworldGenerator, buffer: &mut OwnerBuffer, pos: IVec3, rand: &mut JavaRand) -> bool {
    let height = rand.next_i32_bounded(5) + 7;
    let leaves_offset = height - rand.next_i32_bounded(2) - 3;
    let leaves_height = height - leaves_offset;
    let max_radius = rand.next_i32_bounded(leaves_height + 1);

    let leaves_y = pos.y + leaves_offset;
    let check_radius = |y: i32| if y < leaves_y { 0 } else { max_radius };

    if !check_tree(generator, buffer, pos, height, check_radius) {
        return false;
    }

    let block_ids = &generator.block_ids;
    write(buffer, pos.x, pos.y - 1, pos.z, block_ids.dirt);

    let mut current_radius = 0;
    for wy in leaves_y..=(pos.y + height) {
        for wx in pos.x - current_radius..=pos.x + current_radius {
            for wz in pos.z - current_radius..=pos.z + current_radius {
                let dx = (wx - pos.x).abs();
                let dz = (wz - pos.z).abs();
                if dx != current_radius || dz != current_radius || current_radius <= 0 {
                    let id = read(generator, buffer, wx, wy, wz);
                    if id == block_ids.air || is_leaves(block_ids, id) {
                        write(buffer, wx, wy, wz, block_ids.spruce_leaves);
                    }
                }
            }
        }

        if current_radius >= 1 && wy == leaves_y + 1 {
            current_radius -= 1;
        } else if current_radius < max_radius {
            current_radius += 1;
        }
    }

    for wy in pos.y..(pos.y + height - 1) {
        let id = read(generator, buffer, pos.x, wy, pos.z);
        if id == block_ids.air || is_leaves(block_ids, id) {
            write(buffer, pos.x, wy, pos.z, block_ids.spruce_log);
        }
    }

    true
}

/// A spruce tree with a wider, more irregular canopy (variant 2). Ported from
/// `Spruce2TreeGenerator`.
fn place_spruce2_tree(generator: &OverworldGenerator, buffer: &mut OwnerBuffer, pos: IVec3, rand: &mut JavaRand) -> bool {
    let height = rand.next_i32_bounded(4) + 6;
    let leaves_offset = rand.next_i32_bounded(2) + 1;
    let leaves_height = height - leaves_offset;
    let max_radius = rand.next_i32_bounded(2) + 2;

    let leaves_y = pos.y + leaves_offset;
    let check_radius = |y: i32| if y < leaves_y { 0 } else { max_radius };

    if !check_tree(generator, buffer, pos, height, check_radius) {
        return false;
    }

    let block_ids = &generator.block_ids;
    write(buffer, pos.x, pos.y - 1, pos.z, block_ids.dirt);

    let mut current_radius = rand.next_i32_bounded(2);
    let mut start_radius = 0;
    let mut global_radius = 1;

    for dy in 0..=leaves_height {
        let wy = pos.y + height - dy;

        for wx in pos.x - current_radius..=pos.x + current_radius {
            for wz in pos.z - current_radius..=pos.z + current_radius {
                let dx = (wx - pos.x).abs();
                let dz = (wz - pos.z).abs();
                if dx != current_radius || dz != current_radius || current_radius <= 0 {
                    let id = read(generator, buffer, wx, wy, wz);
                    if id == block_ids.air || is_leaves(block_ids, id) {
                        write(buffer, wx, wy, wz, block_ids.spruce_leaves);
                    }
                }
            }
        }

        if current_radius >= global_radius {
            current_radius = start_radius;
            start_radius = 1;
            global_radius = max_radius.min(global_radius + 1);
        } else {
            current_radius += 1;
        }
    }

    let log_offset = rand.next_i32_bounded(3);
    for wy in pos.y..(pos.y + height - log_offset) {
        let id = read(generator, buffer, pos.x, wy, pos.z);
        if id == block_ids.air || is_leaves(block_ids, id) {
            write(buffer, pos.x, wy, pos.z, block_ids.spruce_log);
        }
    }

    true
}

/// A big oak: a tapering trunk (possibly shortened if it runs into something) with
/// leaf clusters scattered in rings around it, each connected back to the trunk by
/// its own branch. Ported from the reference's `BigTreeGenerator` (its "natural"
/// variant - the only one the reference itself ever calls out of population).
#[derive(Clone, Copy)]
struct BigTreeNode {
    pos: IVec3,
    start_y: i32,
}

//noinspection ALL,RsApproxConstant
fn place_big_tree(generator: &OverworldGenerator, buffer: &mut OwnerBuffer, pos: IVec3, rand: &mut JavaRand) -> bool {
    const HEIGHT_RANGE: i32 = 12;
    const HEIGHT_ATTENUATION: f32 = 0.618;
    const LEAF_DENSITY: f32 = 1.0;
    const BRANCH_DELTA_HEIGHT: i32 = 5;
    const BRANCH_SCALE: f32 = 1.0;
    const BRANCH_SLOPE: f32 = 0.381;

    let block_ids = &generator.block_ids;

    // Reseed happens unconditionally, before the ground check - the reference
    // does this too, and skipping the reseed on a failed check desyncs everything
    // placed after this in the owner's stream.
    let mut rand = JavaRand::new(rand.next_i64());
    let mut height = rand.next_i32_bounded(HEIGHT_RANGE) + 5;

    let below = read(generator, buffer, pos.x, pos.y - 1, pos.z);
    if below != block_ids.grass && below != block_ids.dirt {
        return false;
    }

    if let Some(blocked_at) = check_big_tree_branch(generator, buffer, block_ids, pos, pos + IVec3::new(0, height, 0)) {
        if blocked_at.y - pos.y < 6 {
            return false;
        }
        height = blocked_at.y - pos.y;
    }

    let mut height_attenuated = (height as f32 * HEIGHT_ATTENUATION) as i32;
    if height_attenuated >= height {
        height_attenuated = height - 1;
    }

    let nodes_per_height = ((1.382 + (LEAF_DENSITY * height as f32 / 13.0).powi(2)) as i32).max(1) as usize;
    let mut nodes: Vec<BigTreeNode> = Vec::with_capacity(nodes_per_height * height as usize);

    let mut leaf_offset = height - BRANCH_DELTA_HEIGHT;
    let mut leaf_y = pos.y + leaf_offset;
    let start_y = pos.y + height_attenuated;

    nodes.push(BigTreeNode {
        pos: IVec3::new(pos.x, leaf_y, pos.z),
        start_y,
    });
    leaf_y -= 1;

    while leaf_offset >= 0 {
        let size = calc_big_tree_layer_size(leaf_offset, height);
        if size >= 0.0 {
            for _ in 0..nodes_per_height {
                let length = BRANCH_SCALE * size * (rand.next_float() + 0.328);
                let angle = rand.next_float() * 2.0 * MC_PI;

                let leaf_x = (length * angle.sin() + pos.x as f32 + 0.5).floor() as i32;
                let leaf_z = (length * angle.cos() + pos.z as f32 + 0.5).floor() as i32;
                let leaf_pos = IVec3::new(leaf_x, leaf_y, leaf_z);

                if check_big_tree_branch(generator, buffer, block_ids, leaf_pos, leaf_pos + IVec3::new(0, BRANCH_DELTA_HEIGHT, 0)).is_none() {
                    let horiz_dist = ((pos.x as f32 - leaf_x as f32).powi(2) + (pos.z as f32 - leaf_z as f32).powi(2)).sqrt();
                    let leaf_start_y = ((leaf_y as f32 - horiz_dist * BRANCH_SLOPE) as i32).min(start_y);
                    let leaf_start_pos = IVec3::new(pos.x, leaf_start_y, pos.z);

                    if check_big_tree_branch(generator, buffer, block_ids, leaf_start_pos, leaf_pos).is_none() {
                        nodes.push(BigTreeNode { pos: leaf_pos, start_y: leaf_start_y });
                    }
                }
            }
        }

        leaf_y -= 1;
        leaf_offset -= 1;
    }

    for node in &nodes {
        place_big_tree_leaf(generator, buffer, block_ids, node.pos, BRANCH_DELTA_HEIGHT);
    }

    place_big_tree_branch(buffer, block_ids, pos, pos + IVec3::new(0, height_attenuated, 0));

    let min_height = height as f32 * 0.2;
    for node in &nodes {
        if (node.start_y - pos.y) as f32 >= min_height {
            place_big_tree_branch(buffer, block_ids, IVec3::new(pos.x, node.start_y, pos.z), node.pos);
        }
    }

    true
}

/// Grows a ball of leaves at `pos`, `branch_delta_height` layers tall, narrower at the
/// very top and bottom than in the middle.
fn place_big_tree_leaf(generator: &OverworldGenerator, buffer: &mut OwnerBuffer, block_ids: &BlockIds, pos: IVec3, branch_delta_height: i32) {
    for dy in 0..branch_delta_height {
        let radius = if dy != 0 && dy != branch_delta_height - 1 { 3.0 } else { 2.0 };
        place_big_tree_leaf_layer(generator, buffer, block_ids, pos + IVec3::new(0, dy, 0), radius);
    }
}

fn place_big_tree_leaf_layer(generator: &OverworldGenerator, buffer: &mut OwnerBuffer, block_ids: &BlockIds, pos: IVec3, radius: f32) {
    let block_radius = (radius + 0.618) as i32;

    for dx in -block_radius..=block_radius {
        for dz in -block_radius..=block_radius {
            let dist = ((dx.abs() as f32 + 0.5).powi(2) + (dz.abs() as f32 + 0.5).powi(2)).sqrt();
            if dist > radius {
                continue;
            }

            let (wx, wz) = (pos.x + dx, pos.z + dz);
            let id = read(generator, buffer, wx, pos.y, wz);
            if id == block_ids.air || is_leaves(block_ids, id) {
                write(buffer, wx, pos.y, wz, block_ids.oak_leaves);
            }
        }
    }
}

/// Draws a log along the straight line from `from` to `to`, unconditionally (unlike
/// every other tree's trunk, which only overwrites air/leaves).
fn place_big_tree_branch(buffer: &mut OwnerBuffer, block_ids: &BlockIds, from: IVec3, to: IVec3) {
    for pos in BlockLineIter::new(from, to) {
        write(buffer, pos.x, pos.y, pos.z, block_ids.oak_log);
    }
}

/// Walks the straight line from `from` to `to` and returns the first position that
/// isn't air or leaves, or `None` if the whole line is clear.
fn check_big_tree_branch(generator: &OverworldGenerator, buffer: &OwnerBuffer, block_ids: &BlockIds, from: IVec3, to: IVec3) -> Option<IVec3> {
    for pos in BlockLineIter::new(from, to) {
        let id = read(generator, buffer, pos.x, pos.y, pos.z);
        if id != block_ids.air && !is_leaves(block_ids, id) {
            return Some(pos);
        }
    }
    None
}

fn calc_big_tree_layer_size(leaf_offset: i32, height: i32) -> f32 {
    if (leaf_offset as f64) < (height as f64 * 0.3) {
        return -1.618;
    }

    let a = height as f32 / 2.0;
    let b = a - leaf_offset as f32;

    (if b == 0.0 {
        a
    } else if b.abs() >= a {
        0.0
    } else {
        (a.abs().powi(2) - b.abs().powi(2)).sqrt()
    }) * 0.5
}

/// Iterates every block position along a straight 3D line from `from` to `to`
/// inclusive, stepping one block at a time along whichever axis has the largest
/// delta and interpolating the other two. Ported from the reference's
/// `BlockLineIter`.
#[derive(Default)]
struct BlockLineIter {
    from: IVec3,
    major_axis: usize,
    second_axis: usize,
    third_axis: usize,
    second_ratio: f32,
    third_ratio: f32,
    major_inc: i32,
    major_max: i32,
    major: i32,
}

impl BlockLineIter {
    fn new(from: IVec3, to: IVec3) -> Self {
        let delta = to - from;
        if delta == IVec3::ZERO {
            return Self::default();
        }

        let major_axis = (0..3).map(|i: usize| (i, delta[i].abs())).max_by_key(|&(_, delta)| delta).unwrap().0;
        let second_axis = (major_axis + 1) % 3;
        let third_axis = (major_axis + 2) % 3;

        let major_delta = delta[major_axis];
        let second_ratio = delta[second_axis] as f32 / major_delta as f32;
        let third_ratio = delta[third_axis] as f32 / major_delta as f32;

        let major_inc = major_delta.signum();
        let major_max = major_delta + major_inc;

        Self {
            from,
            major_axis,
            second_axis,
            third_axis,
            second_ratio,
            third_ratio,
            major_inc,
            major_max,
            major: 0,
        }
    }
}

impl Iterator for BlockLineIter {
    type Item = IVec3;

    fn next(&mut self) -> Option<Self::Item> {
        if self.major == self.major_max {
            return None;
        }

        let mut pos = IVec3::ZERO;
        pos[self.major_axis] = self.from[self.major_axis] + self.major;
        pos[self.second_axis] = (self.from[self.second_axis] as f32 + self.major as f32 * self.second_ratio + 0.5).floor() as i32;
        pos[self.third_axis] = (self.from[self.third_axis] as f32 + self.major as f32 * self.third_ratio + 0.5).floor() as i32;
        self.major += self.major_inc;
        Some(pos)
    }
}
