use super::{CHUNK_HEIGHT, CHUNK_WIDTH};

#[derive(Clone)]
pub struct ChunkBuffer {
    blocks: Box<[i32]>,
}

impl ChunkBuffer {
    pub fn new(air_id: i32) -> Self {
        Self {
            blocks: vec![air_id; CHUNK_WIDTH * CHUNK_HEIGHT * CHUNK_WIDTH].into_boxed_slice(),
        }
    }

    fn index(x: usize, y: usize, z: usize) -> usize {
        (x * CHUNK_HEIGHT + y) * CHUNK_WIDTH + z
    }

    pub fn get(&self, x: usize, y: usize, z: usize) -> i32 {
        self.blocks[Self::index(x, y, z)]
    }

    pub fn set(&mut self, x: usize, y: usize, z: usize, block_id: i32) {
        self.blocks[Self::index(x, y, z)] = block_id;
    }
}
