use glam::{DVec3, IVec3};

use super::{BlockIds, CHUNK_HEIGHT, CHUNK_WIDTH, chunk_seed};
use crate::level::generator::math::{MC_PI, mc_sin, mc_sin_cos};
use crate::level::generator::shared::chunk_buffer::ChunkBuffer;
use crate::rand::java::JavaRand;
use crate::rand::primitives::Bound;

pub struct CaveCarver {
    radius: i32,
}

impl CaveCarver {
    pub fn new(radius: i32) -> Self {
        Self { radius }
    }

    pub fn carve(&self, seed: i64, x: i32, z: i32, column: &mut ChunkBuffer, block_ids: &BlockIds) {
        let mut rand = JavaRand::new(seed);

        for from_x in x - self.radius..=x + self.radius {
            for from_z in z - self.radius..=z + self.radius {
                rand.set_seed(chunk_seed(seed, from_x, from_z));
                self.carve_from(from_x, from_z, x, z, column, &mut rand, block_ids);
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn carve_from(&self, from_x: i32, from_z: i32, x: i32, z: i32, column: &mut ChunkBuffer, rand: &mut JavaRand, block_ids: &BlockIds) {
        let count = rand.random_with::<i32>(Bound::new(10));
        let bound = count + 1;
        let count = rand.random_with::<i32>(Bound::new(bound));
        let bound = count + 1;
        let count = rand.random_with::<i32>(Bound::new(bound));

        if rand.random_with::<i32>(Bound::new(5)) != 0 {
            return;
        }

        for _ in 0..count {
            let bound = CHUNK_WIDTH as i32;
            let bound1 = CHUNK_HEIGHT as i32;
            let bound2 = CHUNK_WIDTH as i32;
            let start = DVec3::new(
                (from_x * CHUNK_WIDTH as i32 + rand.random_with::<i32>(Bound::new(bound2))) as f64,
                rand.random_with::<i32>(Bound::new(bound1)) as f64,
                (from_z * CHUNK_WIDTH as i32 + rand.random_with::<i32>(Bound::new(bound))) as f64,
            );

            let mut node_count = 1;
            if rand.random_with::<i32>(Bound::new(4)) == 0 {
                let start_width = rand.random::<f32>() * 6.0 + 1.0;
                self.carve_node(x, z, column, rand, start, start_width, 0.0, 0.0, -1, -1, 0.5, block_ids);
                node_count += rand.random_with::<i32>(Bound::new(4));
            }

            for _ in 0..node_count {
                let yaw = rand.random::<f32>() * MC_PI * 2.0;
                let pitch = (rand.random::<f32>() - 0.5) * 2.0 / 8.0;
                let start_width = rand.random::<f32>() * 2.0 + rand.random::<f32>();
                self.carve_node(x, z, column, rand, start, start_width * 2.0, yaw, pitch, 0, 0, 0.5, block_ids);
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn carve_node(
        &self,
        x: i32,
        z: i32,
        column: &mut ChunkBuffer,
        chunk_rand: &mut JavaRand,
        mut pos: DVec3,
        start_width: f32,
        mut yaw: f32,
        mut pitch: f32,
        mut offset: i32,
        mut length: i32,
        height_scale: f64,
        block_ids: &BlockIds,
    ) {
        let x_mid = (x * CHUNK_WIDTH as i32 + 8) as f64;
        let z_mid = (z * CHUNK_WIDTH as i32 + 8) as f64;

        let mut rand = JavaRand::new(chunk_rand.random::<i64>());

        if length <= 0 {
            let max_length = self.radius * CHUNK_WIDTH as i32 - CHUNK_WIDTH as i32;
            let bound = max_length / 4;
            length = max_length - rand.random_with::<i32>(Bound::new(bound));
        }

        let auto_offset = offset == -1;
        if auto_offset {
            offset = length / 2;
        }

        let bound = length / 2;
        let branch_offset = rand.random_with::<i32>(Bound::new(bound)) + length / 4;
        let stable_pitch = rand.random_with::<i32>(Bound::new(6)) == 0;

        let mut pitch_scale = 0.0f32;
        let mut yaw_scale = 0.0f32;

        'main: for offset in offset..length {
            let width = 1.5 + (mc_sin(offset as f32 * MC_PI / length as f32) * start_width) as f64;
            let height = width * height_scale;

            let (pitch_sin, pitch_cos) = mc_sin_cos(pitch);
            let (yaw_sin, yaw_cos) = mc_sin_cos(yaw);

            pos.x += (yaw_cos * pitch_cos) as f64;
            pos.y += pitch_sin as f64;
            pos.z += (yaw_sin * pitch_cos) as f64;

            pitch *= if stable_pitch { 0.92 } else { 0.7 };

            pitch += pitch_scale * 0.1;
            yaw += yaw_scale * 0.1;
            pitch_scale *= 0.9;
            yaw_scale *= 12.0 / 16.0;
            pitch_scale += (rand.random::<f32>() - rand.random::<f32>()) * rand.random::<f32>() * 2.0;
            yaw_scale += (rand.random::<f32>() - rand.random::<f32>()) * rand.random::<f32>() * 4.0;

            if !auto_offset && offset == branch_offset && start_width > 1.0 {
                self.carve_node(
                    x,
                    z,
                    column,
                    chunk_rand,
                    pos,
                    rand.random::<f32>() * 0.5 + 0.5,
                    yaw - MC_PI * 0.5,
                    pitch / 3.0,
                    offset,
                    length,
                    1.0,
                    block_ids,
                );
                self.carve_node(
                    x,
                    z,
                    column,
                    chunk_rand,
                    pos,
                    rand.random::<f32>() * 0.5 + 0.5,
                    yaw + MC_PI * 0.5,
                    pitch / 3.0,
                    offset,
                    length,
                    1.0,
                    block_ids,
                );
                return;
            }

            if !auto_offset && rand.random_with::<i32>(Bound::new(4)) == 0 {
                continue;
            }

            let x_mid_delta = pos.x - x_mid;
            let z_mid_delta = pos.z - z_mid;
            let remaining_length = (length - offset) as f64;
            let margin = (start_width + 2.0 + 16.0) as f64;

            if x_mid_delta.powi(2) + z_mid_delta.powi(2) - remaining_length.powi(2) > margin.powi(2) {
                return;
            }

            if pos.x < x_mid - 16.0 - width * 2.0 || pos.z < z_mid - 16.0 - width * 2.0 || pos.x > x_mid + 16.0 + width * 2.0 || pos.z > z_mid + 16.0 + width * 2.0 {
                continue;
            }

            let size = DVec3::new(width, height, width);

            let mut start = (pos - size).floor().as_ivec3();
            let mut end = (pos + size).floor().as_ivec3();

            start -= IVec3::new(x * CHUNK_WIDTH as i32 + 1, 1, z * CHUNK_WIDTH as i32 + 1);
            end -= IVec3::new(x * CHUNK_WIDTH as i32 - 1, -1, z * CHUNK_WIDTH as i32 - 1);

            let start = start.max(IVec3::new(0, 1, 0));
            let end = end.min(IVec3::new(CHUNK_WIDTH as i32, 120, CHUNK_WIDTH as i32));

            // Don't carve near lava: it would just drain out into the cave.
            for bx in start.x..end.x {
                for bz in start.z..end.z {
                    let mut by = end.y + 1;
                    while by >= start.y - 1 {
                        let id = column.get(bx as usize, by as usize, bz as usize);
                        if id == block_ids.lava || id == block_ids.lava_still {
                            continue 'main;
                        } else if by != start.y - 1 && bx != start.x && bx != end.x - 1 && bz != start.z && bz != end.z - 1 {
                            by = start.y;
                        }
                        by -= 1;
                    }
                }
            }

            for bx in start.x..end.x {
                let dx = ((bx + x * CHUNK_WIDTH as i32) as f64 + 0.5 - pos.x) / width;
                for bz in start.z..end.z {
                    let dz = ((bz + z * CHUNK_WIDTH as i32) as f64 + 0.5 - pos.z) / width;

                    let xz_dist_sq = dx.powi(2) + dz.powi(2);
                    if xz_dist_sq >= 1.0 {
                        continue;
                    }

                    for by in (start.y..end.y).rev() {
                        let dy = (by as f64 + 0.5 - pos.y) / height;

                        if dy <= -0.7 || xz_dist_sq + dy.powi(2) >= 1.0 {
                            continue;
                        }

                        let (bx, bz) = (bx as usize, bz as usize);
                        let carve_y = (by + 1) as usize;
                        let prev_id = column.get(bx, carve_y, bz);

                        // The reference also checks dirt/grass here, inherited from the
                        // overworld cave carver this was adapted from - nether terrain
                        // never actually places either, so that's dead code, omitted here.
                        if prev_id == block_ids.netherrack {
                            column.set(bx, carve_y, bz, block_ids.air);
                        }
                    }
                }
            }

            if auto_offset {
                break;
            }
        }
    }
}
