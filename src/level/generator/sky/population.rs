use std::sync::Arc;

use crate::level::generator::shared::quad_chunk_buffer::QuadChunkBuffer;
use crate::level::generator::shared::population::PopulationSource;
pub use crate::level::generator::shared::population::{populate, populate_owner, populate_owner_isolated};
use crate::rand::java::JavaRand;

use super::{SkyGenerator, chunk_seed, dungeon, lake, plant, snow, spring, tree, vein};

impl PopulationSource for SkyGenerator {
    fn owner_population(&self, owner_x: i32, owner_z: i32) -> Arc<QuadChunkBuffer> {
        self.owner_population(owner_x, owner_z)
    }

    fn owner_population_isolated(&self, owner_x: i32, owner_z: i32) -> Arc<QuadChunkBuffer> {
        self.owner_population_isolated(owner_x, owner_z)
    }

    fn run_population(&self, owner_x: i32, owner_z: i32, buffer: &mut QuadChunkBuffer) {
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
