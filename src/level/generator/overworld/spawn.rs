use glam::IVec3;

use super::OverworldGenerator;
use crate::level::generator::shared::CHUNK_HEIGHT;
use crate::level::generator::shared::chunk_buffer::ChunkBuffer;
use crate::rand::java::JavaRand;
use crate::rand::primitives::Bound;

const FIRST_UNCOVERED_SEARCH_Y: usize = 63;
const SPAWN_STEP: i32 = 64;

impl OverworldGenerator {
    pub fn find_spawn(&self) -> IVec3 {
        let mut rand = JavaRand::new(self.seed);
        let mut loaded: Option<((i32, i32), ChunkBuffer)> = None;

        let (mut x, mut z) = (0, 0);
        loop {
            let chunk = (x >> 4, z >> 4);
            if loaded.as_ref().is_none_or(|(at, _)| *at != chunk) {
                loaded = Some((chunk, self.unpopulated_column(chunk.0, chunk.1)));
            }
            let (_, column) = loaded.as_ref().unwrap();
            let (lx, lz) = ((x & 15) as usize, (z & 15) as usize);

            if column.get(lx, self.first_uncovered_y(column, lx, lz), lz) == self.block_ids.sand {
                return IVec3::new(x, self.spawn_feet_y(column, lx, lz), z);
            }

            x += rand.random_with::<i32>(Bound::new(SPAWN_STEP)) - rand.random_with::<i32>(Bound::new(SPAWN_STEP));
            z += rand.random_with::<i32>(Bound::new(SPAWN_STEP)) - rand.random_with::<i32>(Bound::new(SPAWN_STEP));
        }
    }

    fn unpopulated_column(&self, x: i32, z: i32) -> ChunkBuffer {
        let climate = self.climate(x, z);
        let mut column = self.terrain(x, z, &climate);
        self.surface(x, z, &mut column, &climate);
        self.carve_caves(x, z, &mut column);
        column
    }

    fn first_uncovered_y(&self, column: &ChunkBuffer, lx: usize, lz: usize) -> usize {
        let mut y = FIRST_UNCOVERED_SEARCH_Y;
        while y + 1 < CHUNK_HEIGHT && column.get(lx, y + 1, lz) != self.block_ids.air {
            y += 1;
        }
        y
    }

    fn spawn_feet_y(&self, column: &ChunkBuffer, lx: usize, lz: usize) -> i32 {
        let ids = &self.block_ids;
        (1..CHUNK_HEIGHT)
            .rev()
            .find(|&y| {
                let id = column.get(lx, y, lz);
                id != ids.air && id != ids.water && id != ids.water_flowing && id != ids.lava && id != ids.lava_still
            })
            .map_or(-1, |y| y as i32 + 1)
    }
}
