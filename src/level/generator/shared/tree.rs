use glam::IVec3;

use super::block_ids::BlockIds;
use super::material::{height_value, is_opaque_cube};
use super::quad_chunk_buffer::{QuadChunkBuffer, read, write};
use super::{CHUNK_HEIGHT, CHUNK_WIDTH};
use crate::level::generator::overworld::biome::Biome;
use crate::math::floor_double;
use crate::rand::java::JavaRand;
use crate::rand::primitives::Bound;

pub fn populate_from(buffer: &mut QuadChunkBuffer, owner_x: i32, owner_z: i32, biome: Biome, feature_value: f64, block_ids: &BlockIds, rand: &mut JavaRand) {
    let origin = IVec3::new(owner_x * CHUNK_WIDTH as i32, 0, owner_z * CHUNK_WIDTH as i32);
    let base_tree_count = ((feature_value / 8.0 + rand.random::<f64>() * 4.0 + 4.0) / 3.0) as i32;

    let mut tree_count = 0;
    if rand.random_with::<i32>(Bound::new(10)) == 0 {
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

    for _ in 0..tree_count {
        let bound = CHUNK_WIDTH as i32;
        let tree_x = origin.x + rand.random_with::<i32>(Bound::new(bound)) + 8;
        let bound = CHUNK_WIDTH as i32;
        let tree_z = origin.z + rand.random_with::<i32>(Bound::new(bound)) + 8;
        let tree_y = height_value(buffer, block_ids, tree_x, tree_z);
        let pos = IVec3::new(tree_x, tree_y, tree_z);

        match biome {
            Biome::Taiga => {
                if rand.random_with::<i32>(Bound::new(3)) == 0 {
                    place_spruce1_tree(buffer, block_ids, pos, rand);
                } else {
                    place_spruce2_tree(buffer, block_ids, pos, rand);
                }
            }
            Biome::Forest => {
                if rand.random_with::<i32>(Bound::new(5)) == 0 {
                    place_simple_tree(buffer, block_ids, pos, 5, block_ids.birch_log, block_ids.birch_leaves, rand);
                } else if rand.random_with::<i32>(Bound::new(3)) == 0 {
                    place_big_tree(buffer, block_ids, pos, rand);
                } else {
                    place_simple_tree(buffer, block_ids, pos, 4, block_ids.oak_log, block_ids.oak_leaves, rand);
                }
            }
            Biome::RainForest => {
                if rand.random_with::<i32>(Bound::new(3)) == 0 {
                    place_big_tree(buffer, block_ids, pos, rand);
                } else {
                    place_simple_tree(buffer, block_ids, pos, 4, block_ids.oak_log, block_ids.oak_leaves, rand);
                }
            }
            _ => {
                if rand.random_with::<i32>(Bound::new(10)) == 0 {
                    place_big_tree(buffer, block_ids, pos, rand);
                } else {
                    place_simple_tree(buffer, block_ids, pos, 4, block_ids.oak_log, block_ids.oak_leaves, rand);
                }
            }
        }
    }
}

pub fn is_leaves(block_ids: &BlockIds, id: i32) -> bool {
    id == block_ids.oak_leaves || id == block_ids.birch_leaves || id == block_ids.spruce_leaves
}

fn check_tree(buffer: &QuadChunkBuffer, block_ids: &BlockIds, pos: IVec3, height: i32, check_radius: impl Fn(i32) -> i32) -> bool {
    let max_y = pos.y + height + 1;
    if pos.y < 1 || max_y >= CHUNK_HEIGHT as i32 {
        return false;
    }

    let below = read(buffer, pos.x, pos.y - 1, pos.z);
    if below != block_ids.grass && below != block_ids.dirt {
        return false;
    }

    for wy in pos.y..=max_y {
        let radius = check_radius(wy);
        for wx in pos.x - radius..=pos.x + radius {
            for wz in pos.z - radius..=pos.z + radius {
                let id = read(buffer, wx, wy, wz);
                if id == block_ids.air || is_leaves(block_ids, id) {
                    continue;
                }
                return false;
            }
        }
    }

    true
}

#[allow(clippy::too_many_arguments)]
fn place_simple_tree(buffer: &mut QuadChunkBuffer, block_ids: &BlockIds, pos: IVec3, min_height: i32, log_id: i32, leaves_id: i32, rand: &mut JavaRand) -> bool {
    let height = rand.random_with::<i32>(Bound::new(3)) + min_height;

    let check_radius = |y: i32| {
        if y == pos.y {
            0
        } else if y >= pos.y + height - 1 {
            2
        } else {
            1
        }
    };

    if !check_tree(buffer, block_ids, pos, height, check_radius) {
        return false;
    }

    write(buffer, pos.x, pos.y - 1, pos.z, block_ids.dirt);

    for wy in (pos.y + height - 3)..=(pos.y + height) {
        let dy = wy - (pos.y + height);
        let radius = 1 - dy / 2;

        for wx in pos.x - radius..=pos.x + radius {
            for wz in pos.z - radius..=pos.z + radius {
                let dx = (wx - pos.x).abs();
                let dz = (wz - pos.z).abs();
                if (dx != radius || dz != radius || (rand.random_with::<i32>(Bound::new(2)) != 0 && dy != 0)) && !is_opaque_cube(block_ids, read(buffer, wx, wy, wz)) {
                    write(buffer, wx, wy, wz, leaves_id);
                }
            }
        }
    }

    for wy in pos.y..(pos.y + height) {
        let id = read(buffer, pos.x, wy, pos.z);
        if id == block_ids.air || is_leaves(block_ids, id) {
            write(buffer, pos.x, wy, pos.z, log_id);
        }
    }

    true
}

fn place_spruce1_tree(buffer: &mut QuadChunkBuffer, block_ids: &BlockIds, pos: IVec3, rand: &mut JavaRand) -> bool {
    let height = rand.random_with::<i32>(Bound::new(5)) + 7;
    let leaves_offset = height - rand.random_with::<i32>(Bound::new(2)) - 3;
    let leaves_height = height - leaves_offset;
    let bound = leaves_height + 1;
    let max_radius = 1 + rand.random_with::<i32>(Bound::new(bound));

    let leaves_y = pos.y + leaves_offset;
    let check_radius = |y: i32| if y < leaves_y { 0 } else { max_radius };

    if !check_tree(buffer, block_ids, pos, height, check_radius) {
        return false;
    }

    write(buffer, pos.x, pos.y - 1, pos.z, block_ids.dirt);

    let mut current_radius = 0;
    for wy in (leaves_y..=(pos.y + height)).rev() {
        for wx in pos.x - current_radius..=pos.x + current_radius {
            for wz in pos.z - current_radius..=pos.z + current_radius {
                let dx = (wx - pos.x).abs();
                let dz = (wz - pos.z).abs();
                if (dx != current_radius || dz != current_radius || current_radius <= 0) && !is_opaque_cube(block_ids, read(buffer, wx, wy, wz)) {
                    write(buffer, wx, wy, wz, block_ids.spruce_leaves);
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
        let id = read(buffer, pos.x, wy, pos.z);
        if id == block_ids.air || is_leaves(block_ids, id) {
            write(buffer, pos.x, wy, pos.z, block_ids.spruce_log);
        }
    }

    true
}

fn place_spruce2_tree(buffer: &mut QuadChunkBuffer, block_ids: &BlockIds, pos: IVec3, rand: &mut JavaRand) -> bool {
    let height = rand.random_with::<i32>(Bound::new(4)) + 6;
    let leaves_offset = rand.random_with::<i32>(Bound::new(2)) + 1;
    let leaves_height = height - leaves_offset;
    let max_radius = rand.random_with::<i32>(Bound::new(2)) + 2;

    let leaves_y = pos.y + leaves_offset;
    let check_radius = |y: i32| if y < leaves_y { 0 } else { max_radius };

    if !check_tree(buffer, block_ids, pos, height, check_radius) {
        return false;
    }

    write(buffer, pos.x, pos.y - 1, pos.z, block_ids.dirt);

    let mut current_radius = rand.random_with::<i32>(Bound::new(2));
    let mut start_radius = 0;
    let mut global_radius = 1;

    for dy in 0..=leaves_height {
        let wy = pos.y + height - dy;

        for wx in pos.x - current_radius..=pos.x + current_radius {
            for wz in pos.z - current_radius..=pos.z + current_radius {
                let dx = (wx - pos.x).abs();
                let dz = (wz - pos.z).abs();
                if (dx != current_radius || dz != current_radius || current_radius <= 0) && !is_opaque_cube(block_ids, read(buffer, wx, wy, wz)) {
                    write(buffer, wx, wy, wz, block_ids.spruce_leaves);
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

    let log_offset = rand.random_with::<i32>(Bound::new(3));
    for wy in pos.y..(pos.y + height - log_offset) {
        let id = read(buffer, pos.x, wy, pos.z);
        if id == block_ids.air || is_leaves(block_ids, id) {
            write(buffer, pos.x, wy, pos.z, block_ids.spruce_log);
        }
    }

    true
}

#[derive(Clone, Copy)]
struct BigTreeNode {
    pos: IVec3,
    start_y: i32,
}

fn place_big_tree(buffer: &mut QuadChunkBuffer, block_ids: &BlockIds, pos: IVec3, rand: &mut JavaRand) -> bool {
    const HEIGHT_RANGE: i32 = 12;
    const HEIGHT_ATTENUATION: f64 = 0.618;
    const LEAF_DENSITY: f64 = 1.0;
    const BRANCH_DELTA_HEIGHT: i32 = 5;
    const BRANCH_SCALE: f64 = 1.0;
    const BRANCH_SLOPE: f64 = 0.381;
    //noinspection RsApproxConstant
    #[allow(clippy::approx_constant)]
    const BRANCH_ANGLE_PI: f64 = 3.14159;

    let mut rand = JavaRand::new(rand.random::<i64>());
    let mut height = rand.random_with::<i32>(Bound::new(HEIGHT_RANGE)) + 5;

    let below = read(buffer, pos.x, pos.y - 1, pos.z);
    if below != block_ids.grass && below != block_ids.dirt {
        return false;
    }

    if let Some(clear_height) = check_big_tree_branch(buffer, block_ids, pos, pos + IVec3::new(0, height - 1, 0)) {
        if clear_height < 6 {
            return false;
        }
        height = clear_height;
    }

    let mut height_attenuated = (height as f64 * HEIGHT_ATTENUATION) as i32;
    if height_attenuated >= height {
        height_attenuated = height - 1;
    }

    let nodes_per_height = ((1.382 + (LEAF_DENSITY * height as f64 / 13.0).powi(2)) as i32).max(1) as usize;
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
                let length = BRANCH_SCALE * size as f64 * (rand.random::<f32>() as f64 + 0.328);
                let angle = rand.random::<f32>() as f64 * 2.0 * BRANCH_ANGLE_PI;

                let leaf_x = floor_double(length * angle.sin() + pos.x as f64 + 0.5);
                let leaf_z = floor_double(length * angle.cos() + pos.z as f64 + 0.5);
                let leaf_pos = IVec3::new(leaf_x, leaf_y, leaf_z);

                if check_big_tree_branch(buffer, block_ids, leaf_pos, leaf_pos + IVec3::new(0, BRANCH_DELTA_HEIGHT, 0)).is_none() {
                    let horiz_dist = (((pos.x - leaf_x).abs() as f64).powi(2) + ((pos.z - leaf_z).abs() as f64).powi(2)).sqrt();
                    let branch_drop = horiz_dist * BRANCH_SLOPE;
                    let leaf_start_y = if leaf_y as f64 - branch_drop > start_y as f64 {
                        start_y
                    } else {
                        (leaf_y as f64 - branch_drop) as i32
                    };
                    let leaf_start_pos = IVec3::new(pos.x, leaf_start_y, pos.z);

                    if check_big_tree_branch(buffer, block_ids, leaf_start_pos, leaf_pos).is_none() {
                        nodes.push(BigTreeNode { pos: leaf_pos, start_y: leaf_start_y });
                    }
                }
            }
        }

        leaf_y -= 1;
        leaf_offset -= 1;
    }

    for node in &nodes {
        place_big_tree_leaf(buffer, block_ids, node.pos, BRANCH_DELTA_HEIGHT);
    }

    place_big_tree_branch(buffer, block_ids, pos, pos + IVec3::new(0, height_attenuated, 0));

    let min_height = height as f64 * 0.2;
    for node in &nodes {
        if (node.start_y - pos.y) as f64 >= min_height {
            place_big_tree_branch(buffer, block_ids, IVec3::new(pos.x, node.start_y, pos.z), node.pos);
        }
    }

    true
}

fn place_big_tree_leaf(buffer: &mut QuadChunkBuffer, block_ids: &BlockIds, pos: IVec3, branch_delta_height: i32) {
    for dy in 0..branch_delta_height {
        let radius = if dy != 0 && dy != branch_delta_height - 1 { 3.0 } else { 2.0 };
        place_big_tree_leaf_layer(buffer, block_ids, pos + IVec3::new(0, dy, 0), radius);
    }
}

fn place_big_tree_leaf_layer(buffer: &mut QuadChunkBuffer, block_ids: &BlockIds, pos: IVec3, radius: f32) {
    let block_radius = (radius as f64 + 0.618) as i32;

    for dx in -block_radius..=block_radius {
        for dz in -block_radius..=block_radius {
            let dist = ((dx.abs() as f64 + 0.5).powi(2) + (dz.abs() as f64 + 0.5).powi(2)).sqrt();
            if dist > radius as f64 {
                continue;
            }

            let (wx, wz) = (pos.x + dx, pos.z + dz);
            let id = read(buffer, wx, pos.y, wz);
            if id == block_ids.air || is_leaves(block_ids, id) {
                write(buffer, wx, pos.y, wz, block_ids.oak_leaves);
            }
        }
    }
}

fn place_big_tree_branch(buffer: &mut QuadChunkBuffer, block_ids: &BlockIds, from: IVec3, to: IVec3) {
    let Some(line) = BlockLine::new(from, to) else { return };
    for step in line.steps() {
        let pos = line.at(step, 0.5);
        write(buffer, pos.x, pos.y, pos.z, block_ids.oak_log);
    }
}

fn check_big_tree_branch(buffer: &QuadChunkBuffer, block_ids: &BlockIds, from: IVec3, to: IVec3) -> Option<i32> {
    let line = BlockLine::new(from, to)?;
    for step in line.steps() {
        let pos = line.at(step, 0.0);
        let id = read(buffer, pos.x, pos.y, pos.z);
        if id != block_ids.air && !is_leaves(block_ids, id) {
            return Some(step.abs());
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
        ((a.abs() as f64).powi(2) - (b.abs() as f64).powi(2)).sqrt() as f32
    }) * 0.5
}

struct BlockLine {
    from: IVec3,
    major_axis: usize,
    second_axis: usize,
    third_axis: usize,
    second_ratio: f64,
    third_ratio: f64,
    major_inc: i32,
    major_end: i32,
}

impl BlockLine {
    fn new(from: IVec3, to: IVec3) -> Option<Self> {
        let delta = to - from;

        let mut major_axis = 0;
        for axis in 1..3 {
            if delta[axis].abs() > delta[major_axis].abs() {
                major_axis = axis;
            }
        }

        let major_delta = delta[major_axis];
        if major_delta == 0 {
            return None;
        }

        let second_axis = (major_axis + 1) % 3;
        let third_axis = (major_axis + 2) % 3;
        let major_inc = major_delta.signum();

        Some(Self {
            from,
            major_axis,
            second_axis,
            third_axis,
            second_ratio: delta[second_axis] as f64 / major_delta as f64,
            third_ratio: delta[third_axis] as f64 / major_delta as f64,
            major_inc,
            major_end: major_delta + major_inc,
        })
    }

    fn steps(&self) -> impl Iterator<Item = i32> + use<> {
        let inc = self.major_inc;
        (0..self.major_end.abs()).map(move |i| i * inc)
    }

    fn at(&self, step: i32, rounding: f64) -> IVec3 {
        let mut pos = IVec3::ZERO;
        pos[self.major_axis] = self.from[self.major_axis] + step;
        pos[self.second_axis] = floor_double(self.from[self.second_axis] as f64 + step as f64 * self.second_ratio + rounding);
        pos[self.third_axis] = floor_double(self.from[self.third_axis] as f64 + step as f64 * self.third_ratio + rounding);
        pos
    }
}
