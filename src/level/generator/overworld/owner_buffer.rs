use super::column::Column;
use super::{CHUNK_HEIGHT, CHUNK_WIDTH, OverworldGenerator};

/// A 2x2 grid of columns covering owner chunk `(owner_x, owner_z)` and its forward
/// neighbors (`+x`, `+z`, `+x+z`) - the whole area any of its own population features
/// can reach. Built once per owner and reused for every target chunk that needs a
/// slice of it.
pub(super) struct OwnerBuffer {
    owner_x: i32,
    owner_z: i32,
    columns: [[Column; 2]; 2],
}

impl OwnerBuffer {
    pub(super) fn new(generator: &OverworldGenerator, owner_x: i32, owner_z: i32) -> Self {
        let columns = std::array::from_fn(|dx| std::array::from_fn(|dz| (*generator.terrain_and_caves(owner_x + dx as i32, owner_z + dz as i32)).clone()));
        Self { owner_x, owner_z, columns }
    }

    pub(super) fn column(&self, cx: i32, cz: i32) -> Option<&Column> {
        let (dx, dz) = (cx - self.owner_x, cz - self.owner_z);
        if (0..2).contains(&dx) && (0..2).contains(&dz) {
            Some(&self.columns[dx as usize][dz as usize])
        } else {
            None
        }
    }

    fn column_mut(&mut self, cx: i32, cz: i32) -> Option<&mut Column> {
        let (dx, dz) = (cx - self.owner_x, cz - self.owner_z);
        if (0..2).contains(&dx) && (0..2).contains(&dz) {
            Some(&mut self.columns[dx as usize][dz as usize])
        } else {
            None
        }
    }

    /// Used to seed this owner's home chunk from its backward neighbors before its
    /// own population runs - see `population::populate_owner`.
    pub(super) fn set_column(&mut self, cx: i32, cz: i32, column: Column) {
        if let Some(slot) = self.column_mut(cx, cz) {
            *slot = column;
        }
    }
}

pub(super) fn read(generator: &OverworldGenerator, buffer: &OwnerBuffer, wx: i32, wy: i32, wz: i32) -> i32 {
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

pub(super) fn write(buffer: &mut OwnerBuffer, wx: i32, wy: i32, wz: i32, block_id: i32) {
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
