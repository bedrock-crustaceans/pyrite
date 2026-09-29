use super::TerrainSource;
use super::chunk_buffer::ChunkBuffer;
use super::{CHUNK_HEIGHT, CHUNK_WIDTH};

pub struct QuadChunkBuffer {
    x: i32,
    z: i32,
    chunks: [[ChunkBuffer; 2]; 2],
    dirty: [[Vec<(u8, u8, u8)>; 2]; 2],
}

impl QuadChunkBuffer {
    pub fn new(generator: &impl TerrainSource, owner_x: i32, owner_z: i32, home: ChunkBuffer) -> Self {
        let mut home = Some(home);
        let columns = std::array::from_fn(|dx| {
            std::array::from_fn(|dz| {
                if dx == 0 && dz == 0 {
                    home.take().expect("home slot is visited exactly once by array::from_fn")
                } else {
                    (*generator.terrain_and_caves(owner_x + dx as i32, owner_z + dz as i32)).clone()
                }
            })
        });
        Self {
            x: owner_x,
            z: owner_z,
            chunks: columns,
            dirty: Default::default(),
        }
    }

    pub fn column(&self, cx: i32, cz: i32) -> Option<&ChunkBuffer> {
        let (dx, dz) = (cx - self.x, cz - self.z);
        if (0..2).contains(&dx) && (0..2).contains(&dz) {
            Some(&self.chunks[dx as usize][dz as usize])
        } else {
            None
        }
    }

    pub fn column_dirty(&self, cx: i32, cz: i32) -> Option<&[(u8, u8, u8)]> {
        let (dx, dz) = (cx - self.x, cz - self.z);
        if (0..2).contains(&dx) && (0..2).contains(&dz) {
            Some(&self.dirty[dx as usize][dz as usize])
        } else {
            None
        }
    }

    fn column_mut(&mut self, cx: i32, cz: i32) -> Option<(&mut ChunkBuffer, &mut Vec<(u8, u8, u8)>)> {
        let (dx, dz) = (cx - self.x, cz - self.z);
        if (0..2).contains(&dx) && (0..2).contains(&dz) {
            Some((&mut self.chunks[dx as usize][dz as usize], &mut self.dirty[dx as usize][dz as usize]))
        } else {
            None
        }
    }
}

pub struct ColumnCursor {
    cx: i32,
    cz: i32,
    lx: usize,
    lz: usize,
}

pub fn column_at(wx: i32, wz: i32) -> ColumnCursor {
    ColumnCursor {
        cx: wx.div_euclid(CHUNK_WIDTH as i32),
        cz: wz.div_euclid(CHUNK_WIDTH as i32),
        lx: wx.rem_euclid(CHUNK_WIDTH as i32) as usize,
        lz: wz.rem_euclid(CHUNK_WIDTH as i32) as usize,
    }
}

impl ColumnCursor {
    pub fn read(&self, generator: &impl TerrainSource, buffer: &QuadChunkBuffer, wy: i32) -> i32 {
        if !(0..CHUNK_HEIGHT as i32).contains(&wy) {
            return -1;
        }

        match buffer.column(self.cx, self.cz) {
            Some(column) => column.get(self.lx, wy as usize, self.lz),
            None => generator.terrain_and_caves(self.cx, self.cz).get(self.lx, wy as usize, self.lz),
        }
    }

    pub fn write(&self, buffer: &mut QuadChunkBuffer, wy: i32, block_id: i32) {
        if !(0..CHUNK_HEIGHT as i32).contains(&wy) {
            return;
        }

        if let Some((column, dirty)) = buffer.column_mut(self.cx, self.cz) {
            column.set(self.lx, wy as usize, self.lz, block_id);
            dirty.push((self.lx as u8, wy as u8, self.lz as u8));
        }
    }
}

pub fn read(generator: &impl TerrainSource, buffer: &QuadChunkBuffer, wx: i32, wy: i32, wz: i32) -> i32 {
    column_at(wx, wz).read(generator, buffer, wy)
}

pub fn write(buffer: &mut QuadChunkBuffer, wx: i32, wy: i32, wz: i32, block_id: i32) {
    column_at(wx, wz).write(buffer, wy, block_id)
}
