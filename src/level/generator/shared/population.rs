use std::sync::Arc;

use super::quad_chunk_buffer::QuadChunkBuffer;
use super::{ChunkBuffer, TerrainSource};

pub trait PopulationSource: TerrainSource {
    fn owner_population(&self, owner_x: i32, owner_z: i32) -> Option<Arc<QuadChunkBuffer>>;
    fn run_population(&self, owner_x: i32, owner_z: i32, buffer: &mut QuadChunkBuffer);
}

pub fn populate_owner<G: PopulationSource>(generator: &G, owner_x: i32, owner_z: i32) -> QuadChunkBuffer {
    let home = backward_seed(generator, owner_x, owner_z);
    let mut buffer = QuadChunkBuffer::new(generator, owner_x, owner_z, home);
    generator.run_population(owner_x, owner_z, &mut buffer);
    buffer
}

fn backward_seed<G: PopulationSource>(generator: &G, owner_x: i32, owner_z: i32) -> ChunkBuffer {
    let baseline = generator.terrain_and_caves(owner_x, owner_z);
    let mut column = (*baseline).clone();

    for (backward_x, backward_z) in [(owner_x - 1, owner_z - 1), (owner_x - 1, owner_z), (owner_x, owner_z - 1)] {
        if let Some(backward_buffer) = generator.owner_population(backward_x, backward_z) {
            overlay_owner(&baseline, &backward_buffer, owner_x, owner_z, &mut column);
        }
    }

    column
}

fn overlay_owner(baseline: &ChunkBuffer, owner_buffer: &QuadChunkBuffer, x: i32, z: i32, column: &mut ChunkBuffer) {
    let Some(owner_column) = owner_buffer.column(x, z) else {
        return;
    };
    let Some(dirty) = owner_buffer.column_dirty(x, z) else {
        return;
    };

    for &(lx, ly, lz) in dirty {
        let (lx, ly, lz) = (lx as usize, ly as usize, lz as usize);
        let owner_value = owner_column.get(lx, ly, lz);
        if owner_value != baseline.get(lx, ly, lz) {
            column.set(lx, ly, lz, owner_value);
        }
    }
}

pub fn populate<G: PopulationSource>(generator: &G, x: i32, z: i32, column: &mut ChunkBuffer) {
    let baseline = generator.terrain_and_caves(x, z);

    for owner_x in x - 1..=x {
        for owner_z in z - 1..=z {
            if let Some(owner_buffer) = generator.owner_population(owner_x, owner_z) {
                overlay_owner(&baseline, &owner_buffer, x, z, column);
            }
        }
    }
}
