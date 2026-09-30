use std::cell::Cell;
use std::fmt::{Display, Formatter};

use super::chunk_buffer::ChunkBuffer;
use super::{CHUNK_HEIGHT, CHUNK_WIDTH};

#[derive(Debug, Clone, Copy)]
pub enum QuadAccess {
    Read,
    Write,
}

#[derive(Debug, Clone, Copy)]
pub struct OutsideQuad {
    pub access: QuadAccess,
    pub chunk: (i32, i32),
    pub owner: (i32, i32),
}

impl Display for OutsideQuad {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        let access = match self.access {
            QuadAccess::Read => "read",
            QuadAccess::Write => "write",
        };
        write!(f, "population {access} in chunk {:?} lies outside the quad owned by {:?}", self.chunk, self.owner)
    }
}

impl std::error::Error for OutsideQuad {}

pub struct QuadChunkBuffer {
    x: i32,
    z: i32,
    chunks: [[ChunkBuffer; 2]; 2],
    dirty: [[Vec<(u8, u8, u8)>; 2]; 2],
    outside: Cell<Option<OutsideQuad>>,
}

impl QuadChunkBuffer {
    pub fn new(owner_x: i32, owner_z: i32, columns: [[ChunkBuffer; 2]; 2]) -> Self {
        Self {
            x: owner_x,
            z: owner_z,
            chunks: columns,
            dirty: Default::default(),
            outside: Cell::new(None),
        }
    }

    pub fn into_changes(self) -> Result<QuadChanges, OutsideQuad> {
        if let Some(outside) = self.outside.get() {
            return Err(outside);
        }

        let changes = std::array::from_fn(|dx| {
            std::array::from_fn(|dz| {
                let column = &self.chunks[dx][dz];
                self.dirty[dx][dz].iter().map(|&(lx, ly, lz)| (lx, ly, lz, column.get(lx as usize, ly as usize, lz as usize))).collect()
            })
        });
        Ok(QuadChanges { x: self.x, z: self.z, changes })
    }

    fn column(&self, cx: i32, cz: i32) -> Option<&ChunkBuffer> {
        let (dx, dz) = (cx - self.x, cz - self.z);
        if (0..2).contains(&dx) && (0..2).contains(&dz) {
            Some(&self.chunks[dx as usize][dz as usize])
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

pub struct QuadChanges {
    x: i32,
    z: i32,
    changes: [[Vec<(u8, u8, u8, i32)>; 2]; 2],
}

impl QuadChanges {
    pub fn overlay_onto(&self, cx: i32, cz: i32, baseline: &ChunkBuffer, target: &mut ChunkBuffer) {
        let (dx, dz) = (cx - self.x, cz - self.z);
        if !(0..2).contains(&dx) || !(0..2).contains(&dz) {
            return;
        }

        for &(lx, ly, lz, value) in &self.changes[dx as usize][dz as usize] {
            let (lx, ly, lz) = (lx as usize, ly as usize, lz as usize);
            if value != baseline.get(lx, ly, lz) {
                target.set(lx, ly, lz, value);
            }
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
    pub fn read(&self, buffer: &QuadChunkBuffer, wy: i32) -> i32 {
        if !(0..CHUNK_HEIGHT as i32).contains(&wy) {
            return -1;
        }

        match buffer.column(self.cx, self.cz) {
            Some(column) => column.get(self.lx, wy as usize, self.lz),
            None => {
                self.record_outside(buffer, QuadAccess::Read);
                -1
            }
        }
    }

    pub fn write(&self, buffer: &mut QuadChunkBuffer, wy: i32, block_id: i32) {
        if !(0..CHUNK_HEIGHT as i32).contains(&wy) {
            return;
        }

        let Some((column, dirty)) = buffer.column_mut(self.cx, self.cz) else {
            self.record_outside(buffer, QuadAccess::Write);
            return;
        };
        column.set(self.lx, wy as usize, self.lz, block_id);
        dirty.push((self.lx as u8, wy as u8, self.lz as u8));
    }

    #[cold]
    #[inline(never)]
    fn record_outside(&self, buffer: &QuadChunkBuffer, access: QuadAccess) {
        if buffer.outside.get().is_none() {
            buffer.outside.set(Some(OutsideQuad {
                access,
                chunk: (self.cx, self.cz),
                owner: (buffer.x, buffer.z),
            }));
        }
    }
}

pub fn read(buffer: &QuadChunkBuffer, wx: i32, wy: i32, wz: i32) -> i32 {
    column_at(wx, wz).read(buffer, wy)
}

pub fn write(buffer: &mut QuadChunkBuffer, wx: i32, wy: i32, wz: i32, block_id: i32) {
    column_at(wx, wz).write(buffer, wy, block_id)
}
