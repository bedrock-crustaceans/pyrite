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

    pub fn take(self) -> Box<[[[[i32; SUB_CHUNK_SIZE]; SUB_CHUNK_SIZE]; SUB_CHUNK_SIZE]; SUB_CHUNK_COUNT]> {
        self.0
    }
}
