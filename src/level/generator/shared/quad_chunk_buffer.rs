use super::TerrainSource;
use super::chunk_buffer::ChunkBuffer;
use super::{CHUNK_HEIGHT, CHUNK_WIDTH};

pub struct QuadChunkBuffer {
    x: i32,
    z: i32,
    chunks: [[ChunkBuffer; 2]; 2],
}

impl QuadChunkBuffer {
    pub fn new(generator: &impl TerrainSource, owner_x: i32, owner_z: i32) -> Self {
        let columns = std::array::from_fn(|dx| std::array::from_fn(|dz| (*generator.terrain_and_caves(owner_x + dx as i32, owner_z + dz as i32)).clone()));
        Self { x: owner_x, z: owner_z, chunks: columns }
    }

    pub fn column(&self, cx: i32, cz: i32) -> Option<&ChunkBuffer> {
        let (dx, dz) = (cx - self.x, cz - self.z);
        if (0..2).contains(&dx) && (0..2).contains(&dz) {
            Some(&self.chunks[dx as usize][dz as usize])
        } else {
            None
        }
    }

    fn column_mut(&mut self, cx: i32, cz: i32) -> Option<&mut ChunkBuffer> {
        let (dx, dz) = (cx - self.x, cz - self.z);
        if (0..2).contains(&dx) && (0..2).contains(&dz) {
            Some(&mut self.chunks[dx as usize][dz as usize])
        } else {
            None
        }
    }

    pub fn set_column(&mut self, cx: i32, cz: i32, column: ChunkBuffer) {
        if let Some(slot) = self.column_mut(cx, cz) {
            *slot = column;
        }
    }
}

pub fn read(generator: &impl TerrainSource, buffer: &QuadChunkBuffer, wx: i32, wy: i32, wz: i32) -> i32 {
    if !(0..CHUNK_HEIGHT as i32).contains(&wy) {
        return -1;
    }

    let cx = wx.div_euclid(CHUNK_WIDTH as i32);
    let cz = wz.div_euclid(CHUNK_WIDTH as i32);
    let lx = wx.rem_euclid(CHUNK_WIDTH as i32) as usize;
    let lz = wz.rem_euclid(CHUNK_WIDTH as i32) as usize;

    match buffer.column(cx, cz) {
        Some(column) => column.get(lx, wy as usize, lz),
        None => generator.terrain_and_caves(cx, cz).get(lx, wy as usize, lz),
    }
}

pub fn write(buffer: &mut QuadChunkBuffer, wx: i32, wy: i32, wz: i32, block_id: i32) {
    if !(0..CHUNK_HEIGHT as i32).contains(&wy) {
        return;
    }

    let cx = wx.div_euclid(CHUNK_WIDTH as i32);
    let cz = wz.div_euclid(CHUNK_WIDTH as i32);
    let lx = wx.rem_euclid(CHUNK_WIDTH as i32) as usize;
    let lz = wz.rem_euclid(CHUNK_WIDTH as i32) as usize;

    if let Some(column) = buffer.column_mut(cx, cz) {
        column.set(lx, wy as usize, lz, block_id);
    }
}
