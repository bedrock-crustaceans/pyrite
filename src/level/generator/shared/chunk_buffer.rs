use chorus::level::chunk::Chunk;
use chorus::level::dimension_type::DimensionType;
use chorus::level::generator::pos::ChunkPos;
use chorus::level::sub_chunk::SubChunk;

use super::{SUB_CHUNK_COUNT, SUB_CHUNK_SIZE};

#[derive(Clone)]
pub struct ChunkBuffer(Box<[[[[i32; SUB_CHUNK_SIZE]; SUB_CHUNK_SIZE]; SUB_CHUNK_SIZE]; SUB_CHUNK_COUNT]>);

impl ChunkBuffer {
    pub fn new(air_id: i32) -> Self {
        Self(Box::new([[[[air_id; SUB_CHUNK_SIZE]; SUB_CHUNK_SIZE]; SUB_CHUNK_SIZE]; SUB_CHUNK_COUNT]))
    }

    pub fn get(&self, x: usize, y: usize, z: usize) -> i32 {
        self.0[y / SUB_CHUNK_SIZE][x][y % SUB_CHUNK_SIZE][z]
    }

    pub fn set(&mut self, x: usize, y: usize, z: usize, block_id: i32) {
        self.0[y / SUB_CHUNK_SIZE][x][y % SUB_CHUNK_SIZE][z] = block_id;
    }

    pub fn into_chunk(self, cell: ChunkPos, dimension_type: DimensionType, air_id: i32, biome: i32) -> Chunk {
        let mut chunk = Chunk::new(cell.x, cell.z, dimension_type.min_sub_chunk_y(), dimension_type.sub_chunk_count(), air_id, biome);

        for (sub_y, blocks) in self.0.iter().enumerate() {
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
}
