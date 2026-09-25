use crate::level::generator::shared::phases::Population;
use crate::level::generator::shared::quad_chunk_buffer::QuadChunkBuffer;
use crate::rand::java::JavaRand;

use super::{SkyGenerator, chunk_seed, dungeon, lake, plant, snow, spring, tree, vein};

impl Population for SkyGenerator {
    fn run_population_raw(&self, owner_x: i32, owner_z: i32, buffer: &mut QuadChunkBuffer) {
        let mut rand = JavaRand::new(chunk_seed(self.seed, owner_x, owner_z));

        lake::populate_from(self, buffer, owner_x, owner_z, &self.block_ids, &mut rand);
        dungeon::populate_from(self, buffer, owner_x, owner_z, &self.block_ids, &mut rand);
        vein::populate_from(self, buffer, owner_x, owner_z, &mut rand);
        tree::populate_from(self, buffer, owner_x, owner_z, &self.block_ids, &mut rand);
        plant::populate_from(self, buffer, owner_x, owner_z, &mut rand);
        spring::populate_from(self, buffer, owner_x, owner_z, &self.block_ids, &mut rand);
        snow::populate_from(self, buffer, owner_x, owner_z, &self.block_ids);
    }
}
