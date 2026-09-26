use super::{CHUNK_HEIGHT, CHUNK_WIDTH};

#[derive(Clone)]
pub struct ChunkBuffer(Box<[[[i32; CHUNK_WIDTH]; CHUNK_HEIGHT]; CHUNK_WIDTH]>);

impl ChunkBuffer {
    pub fn new(air_id: i32) -> Self {
        Self(Box::new([[[air_id; CHUNK_WIDTH]; CHUNK_HEIGHT]; CHUNK_WIDTH]))
    }

    pub fn get(&self, x: usize, y: usize, z: usize) -> i32 {
        self.0[x][y][z]
    }

    pub fn set(&mut self, x: usize, y: usize, z: usize, block_id: i32) {
        self.0[x][y][z] = block_id;
    }
}
