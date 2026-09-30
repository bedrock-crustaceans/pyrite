use glam::IVec3;

use super::block_ids::BlockIds;
use super::material::{is_liquid, is_solid};
use super::quad_chunk_buffer::{ColumnCursor, QuadChunkBuffer, column_at};
use super::{CHUNK_HEIGHT, CHUNK_WIDTH, SNOW_TEMPERATURE_REFERENCE_HEIGHT};

pub fn populate_from(buffer: &mut QuadChunkBuffer, owner_x: i32, owner_z: i32, block_ids: &BlockIds, temperature_at: impl Fn(i32, i32) -> f64) {
    let origin = IVec3::new(owner_x * CHUNK_WIDTH as i32, 0, owner_z * CHUNK_WIDTH as i32);

    for dx in 0..CHUNK_WIDTH as i32 {
        let wx = origin.x + 8 + dx;
        for dz in 0..CHUNK_WIDTH as i32 {
            let wz = origin.z + 8 + dz;
            let column = column_at(wx, wz);

            let Some(height) = top_solid_or_liquid_height(buffer, block_ids, &column) else {
                continue;
            };
            if !(1..CHUNK_HEIGHT as i32).contains(&height) {
                continue;
            }

            let temperature = temperature_at(wx, wz);
            let adjusted_temperature = temperature - (height - SNOW_TEMPERATURE_REFERENCE_HEIGHT) as f64 / 64.0 * 0.3;
            if adjusted_temperature >= 0.5 {
                continue;
            }

            if column.read(buffer, height) != block_ids.air {
                continue;
            }

            let below = column.read(buffer, height - 1);
            if !is_solid(block_ids, below) || below == block_ids.ice {
                continue;
            }

            column.write(buffer, height, block_ids.snow_layer);
        }
    }
}

fn top_solid_or_liquid_height(buffer: &QuadChunkBuffer, block_ids: &BlockIds, column: &ColumnCursor) -> Option<i32> {
    for wy in (1..CHUNK_HEIGHT as i32).rev() {
        let id = column.read(buffer, wy);
        if is_solid(block_ids, id) || is_liquid(block_ids, id) {
            return Some(wy + 1);
        }
    }
    None
}
