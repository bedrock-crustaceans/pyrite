//! Small helpers shared by every dimension's phase wiring: owner-quad cell-position math, and the
//! (not phase-specific) final chunk assembly step. Deliberately *not* home to any logic generic
//! over phase types - each dimension's own `phases` module declares its own concrete phase types
//! and its own view bridging them back to `TerrainSource`/`PopulationSource`, so that dimension's
//! whole pipeline - what each phase requires, and what it actually does - is readable end to end
//! in that one file, without jumping through shared generic plumbing to follow it.

use chorus::level::chunk::Chunk;
use chorus::level::generator::pos::ChunkPos;

use super::chunk_buffer::ChunkBuffer;
use super::quad_chunk_buffer::QuadChunkBuffer;
use super::{SUB_CHUNK_COUNT, SUB_CHUNK_SIZE, SubChunkBlocks, TerrainSource, insert_sub_chunk};

/// A dimension's generator provides its own decoration pass over an owner's forward quad on top
/// of `TerrainSource` (raw terrain+cave generation, plus the fixed config needed to build the
/// final `Chunk` - shared with the decoration code, which reads terrain the same way).
pub trait Population: TerrainSource + Send + Sync + 'static {
    fn run_population_raw(&self, owner_x: i32, owner_z: i32, buffer: &mut QuadChunkBuffer);
}

pub fn backward_neighbors(owner: ChunkPos) -> Vec<ChunkPos> {
    vec![ChunkPos::new(owner.x - 1, owner.z - 1), ChunkPos::new(owner.x - 1, owner.z), ChunkPos::new(owner.x, owner.z - 1)]
}

/// The backward-looking quad a column's population depends on: the column itself plus its three
/// backward neighbors.
pub fn owner_quad(chunk: ChunkPos) -> Vec<ChunkPos> {
    vec![ChunkPos::new(chunk.x - 1, chunk.z - 1), ChunkPos::new(chunk.x - 1, chunk.z), ChunkPos::new(chunk.x, chunk.z - 1), chunk]
}

/// The forward-looking quad an owner's own `QuadChunkBuffer` covers (`owner..=owner+1` in both
/// axes) - the shape `QuadChunkBuffer::new` actually builds, distinct from `owner_quad` above.
pub fn forward_quad(owner: ChunkPos) -> Vec<ChunkPos> {
    vec![owner, ChunkPos::new(owner.x + 1, owner.z), ChunkPos::new(owner.x, owner.z + 1), ChunkPos::new(owner.x + 1, owner.z + 1)]
}

/// Assembles a fully-decorated column into the Bedrock-protocol `Chunk` shape. The same for every
/// dimension - it's just copying already-computed blocks into sub-chunks, nothing generator- or
/// phase-specific about it.
pub fn assemble_chunk(generator: &impl TerrainSource, cell: ChunkPos, column: &ChunkBuffer) -> Chunk {
    let mut chunk = Chunk::new(
        cell.x,
        cell.z,
        generator.min_sub_chunk_y(),
        generator.dimension_sub_chunk_count(),
        generator.air_id(),
        generator.biome(),
    );

    for sub_y in 0..SUB_CHUNK_COUNT as i8 {
        let base_y = sub_y as usize * SUB_CHUNK_SIZE;
        let mut blocks = [[[0i32; SUB_CHUNK_SIZE]; SUB_CHUNK_SIZE]; SUB_CHUNK_SIZE];

        for (lx, plane) in blocks.iter_mut().enumerate() {
            for (ly, row) in plane.iter_mut().enumerate() {
                for (lz, block_id) in row.iter_mut().enumerate() {
                    *block_id = column.get(lx, base_y + ly, lz);
                }
            }
        }

        insert_sub_chunk(&mut chunk, sub_y, &SubChunkBlocks { blocks }, generator.air_id(), generator.biome());
    }

    chunk
}
