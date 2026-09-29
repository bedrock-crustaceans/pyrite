use super::TerrainSource;
use super::chunk_buffer::ChunkBuffer;
use super::quad_chunk_buffer::QuadChunkBuffer;
use chorus::level::chunk::Chunk;
use chorus::level::generator::pos::ChunkPos;
use chorus::level::sub_chunk::SubChunk;

pub trait Population: TerrainSource + Send + Sync + 'static {
    fn run_population_raw(&self, owner_x: i32, owner_z: i32, buffer: &mut QuadChunkBuffer);
}

pub fn backward_neighbors(owner: ChunkPos) -> Vec<ChunkPos> {
    vec![ChunkPos::new(owner.x - 1, owner.z - 1), ChunkPos::new(owner.x - 1, owner.z), ChunkPos::new(owner.x, owner.z - 1)]
}

pub fn owner_quad(chunk: ChunkPos) -> Vec<ChunkPos> {
    vec![ChunkPos::new(chunk.x - 1, chunk.z - 1), ChunkPos::new(chunk.x - 1, chunk.z), ChunkPos::new(chunk.x, chunk.z - 1), chunk]
}

// forward_quad covers owner..=owner+1 in both axes - distinct from owner_quad above, which is
// owner-1..=owner.
pub fn forward_quad(owner: ChunkPos) -> Vec<ChunkPos> {
    vec![owner, ChunkPos::new(owner.x + 1, owner.z), ChunkPos::new(owner.x, owner.z + 1), ChunkPos::new(owner.x + 1, owner.z + 1)]
}

pub fn assemble_chunk(generator: &impl TerrainSource, cell: ChunkPos, column: ChunkBuffer) -> Chunk {
    let air_id = generator.air_id();
    let biome = generator.biome();

    let mut chunk = Chunk::new(cell.x, cell.z, generator.min_sub_chunk_y(), generator.dimension_sub_chunk_count(), air_id, biome);

    let sub_chunks = column.take();
    for (sub_y, blocks) in sub_chunks.iter().enumerate() {
        let mut flat = [0i32; 4096];
        for (lx, plane) in blocks.iter().enumerate() {
            for (ly, row) in plane.iter().enumerate() {
                for (lz, id) in row.iter().enumerate() {
                    flat[SubChunk::index(lx as u8, ly as u8, lz as u8)] = *id;
                }
            }
        }

        if let Some(existing) = chunk.get_sub_chunk_mut(sub_y as i8) {
            *existing = SubChunk::from_blocks(&flat, air_id, biome);
        }
    }

    chunk
}
