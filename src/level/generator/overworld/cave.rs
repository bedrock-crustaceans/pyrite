use glam::{DVec3, IVec3};

use crate::level::generator::java_rand::JavaRand;
use crate::level::generator::math::{MC_PI, mc_sin, mc_sin_cos};

use super::column::Column;
use super::{BlockIds, CHUNK_WIDTH, owner_chunk_seed};

/// Carves caves through a chunk column, ported from the beta 1.7.3 cave carver.
///
/// A cave can start in any chunk within `radius` of the target chunk and wander into it,
/// so every one of those chunks gets its own deterministic seed and is walked in turn.
pub struct CaveCarver {
    radius: i32,
}

impl CaveCarver {
    pub fn new(radius: i32) -> Self {
        Self { radius }
    }

    pub fn carve(&self, seed: i64, x: i32, z: i32, column: &mut Column, block_ids: &BlockIds) {
        let mut rand = JavaRand::new(seed);

        for from_x in x - self.radius..=x + self.radius {
            for from_z in z - self.radius..=z + self.radius {
                rand.set_seed(owner_chunk_seed(seed, from_x, from_z));
                self.carve_from(from_x, from_z, x, z, column, &mut rand, block_ids);
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn carve_from(&self, from_x: i32, from_z: i32, x: i32, z: i32, column: &mut Column, rand: &mut JavaRand, block_ids: &BlockIds) {
        let count = rand.next_i32_bounded(40);
        let count = rand.next_i32_bounded(count + 1);
        let count = rand.next_i32_bounded(count + 1);

        if rand.next_i32_bounded(15) != 0 {
            return;
        }

        for _ in 0..count {
            let start = DVec3::new(
                (from_x * CHUNK_WIDTH as i32 + rand.next_i32_bounded(CHUNK_WIDTH as i32)) as f64,
                {
                    let v = rand.next_i32_bounded(120);
                    rand.next_i32_bounded(v + 8) as f64
                },
                (from_z * CHUNK_WIDTH as i32 + rand.next_i32_bounded(CHUNK_WIDTH as i32)) as f64,
            );

            let mut node_count = 1;
            if rand.next_i32_bounded(4) == 0 {
                let start_width = rand.next_float() * 6.0 + 1.0;
                self.carve_node(x, z, column, rand, start, start_width, 0.0, 0.0, -1, -1, 0.5, block_ids);
                node_count += rand.next_i32_bounded(4);
            }

            for _ in 0..node_count {
                let yaw = rand.next_float() * MC_PI * 2.0;
                let pitch = (rand.next_float() - 0.5) * 2.0 / 8.0;
                let start_width = rand.next_float() * 2.0 + rand.next_float();
                self.carve_node(x, z, column, rand, start, start_width, yaw, pitch, 0, 0, 1.0, block_ids);
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn carve_node(
        &self,
        x: i32,
        z: i32,
        column: &mut Column,
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

        let mut rand = JavaRand::new(chunk_rand.next_i64());

        // The length is the maximum length of the cave from start point to any end.
        if length <= 0 {
            let max_length = self.radius * CHUNK_WIDTH as i32 - CHUNK_WIDTH as i32;
            length = max_length - rand.next_i32_bounded(max_length / 4);
        }

        // The offset is the current generation point in the length of the cave.
        let auto_offset = offset == -1;
        if auto_offset {
            offset = length / 2;
        }

        let branch_offset = rand.next_i32_bounded(length / 2) + length / 4;
        let stable_pitch = rand.next_i32_bounded(6) == 0;

        let mut pitch_scale = 0.0f32;
        let mut yaw_scale = 0.0f32;

        'main: for offset in offset..length {
            // Widen/narrow the tunnel near its ends.
            let width = 1.5 + (mc_sin(offset as f32 * MC_PI / length as f32) * start_width) as f64;
            let height = width * height_scale;

            let (pitch_sin, pitch_cos) = mc_sin_cos(pitch);
            let (yaw_sin, yaw_cos) = mc_sin_cos(yaw);

            pos.x += (yaw_cos * pitch_cos) as f64;
            pos.y += pitch_sin as f64;
            pos.z += (yaw_sin * pitch_cos) as f64;

            // Stabilize pitch around 0 degrees to keep the cave mostly horizontal.
            pitch *= if stable_pitch { 0.92 } else { 0.7 };

            pitch += pitch_scale * 0.1;
            yaw += yaw_scale * 0.1;
            pitch_scale *= 0.9;
            yaw_scale *= 12.0 / 16.0;
            pitch_scale += (rand.next_float() - rand.next_float()) * rand.next_float() * 2.0;
            yaw_scale += (rand.next_float() - rand.next_float()) * rand.next_float() * 4.0;

            // Branch into two perpendicular tunnels at the branch point.
            if !auto_offset && offset == branch_offset && start_width > 1.0 {
                self.carve_node(
                    x,
                    z,
                    column,
                    chunk_rand,
                    pos,
                    rand.next_float() * 0.5 + 0.5,
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
                    rand.next_float() * 0.5 + 0.5,
                    yaw + MC_PI * 0.5,
                    pitch / 3.0,
                    offset,
                    length,
                    1.0,
                    block_ids,
                );
                return;
            }

            if !auto_offset && rand.next_i32_bounded(4) == 0 {
                continue;
            }

            let x_mid_delta = pos.x - x_mid;
            let z_mid_delta = pos.z - z_mid;
            let remaining_length = (length - offset) as f64;
            let margin = (start_width + 2.0 + 16.0) as f64;

            // Abort early once we're too far from the target chunk to ever reach it.
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

            // Don't carve anywhere near water: it would just drain out into the cave.
            // `end.y` is clamped to 120 above, so `by` never reaches CHUNK_HEIGHT here.
            for bx in start.x..end.x {
                for bz in start.z..end.z {
                    let mut by = end.y + 1;
                    while by >= start.y - 1 {
                        if column.get(bx as usize, by as usize, bz as usize) == block_ids.water {
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

                    // The tunnel cross-section is a cylinder in x/z...
                    let xz_dist_sq = dx.powi(2) + dz.powi(2);
                    if xz_dist_sq >= 1.0 {
                        continue;
                    }

                    let mut carving_surface = false;

                    for by in (start.y..end.y).rev() {
                        let dy = (by as f64 + 0.5 - pos.y) / height;

                        // ...capped into a ball at top and bottom.
                        if dy <= -0.7 || xz_dist_sq + dy.powi(2) >= 1.0 {
                            continue;
                        }

                        let (bx, bz) = (bx as usize, bz as usize);
                        
                        let carve_y = (by + 1) as usize;
                        let prev_id = column.get(bx, carve_y, bz);

                        if prev_id == block_ids.grass {
                            carving_surface = true;
                        }

                        if prev_id == block_ids.stone || prev_id == block_ids.dirt || prev_id == block_ids.grass {
                            if by < 10 {
                                // Below y 10, place flowing lava so it keeps spreading via
                                // random ticks, matching the reference behavior.
                                column.set(bx, carve_y, bz, block_ids.lava);
                            } else {
                                column.set(bx, carve_y, bz, block_ids.air);
                                if carving_surface && carve_y > 0 && column.get(bx, carve_y - 1, bz) == block_ids.dirt {
                                    column.set(bx, carve_y - 1, bz, block_ids.grass);
                                }
                            }
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
