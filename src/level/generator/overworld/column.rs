use super::{CHUNK_HEIGHT, CHUNK_WIDTH};

/// A full-height, single-chunk-column scratch buffer. Terrain, surface and cave carving
/// all read and write through this before the result is sliced into subchunks.
#[derive(Clone)]
pub struct Column {
    blocks: Box<[i32]>,
}

impl Column {
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
