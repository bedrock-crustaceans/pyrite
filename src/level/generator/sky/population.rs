use crate::level::generator::shared::quad_chunk_buffer::QuadChunkBuffer;
use crate::rand::java::JavaRand;

use super::{CHUNK_WIDTH, SkyGenerator, chunk_seed, dungeon, lake, plant, snow, spring, tree, vein};

impl SkyGenerator {
    pub(super) fn populate(&self, owner_x: i32, owner_z: i32, buffer: &mut QuadChunkBuffer) {
        let mut rand = JavaRand::new(chunk_seed(self.seed, owner_x, owner_z));

        lake::populate_from(buffer, owner_x, owner_z, &self.block_ids, &mut rand);
        dungeon::populate_from(buffer, owner_x, owner_z, &self.block_ids, &mut rand);
        vein::populate_from(self, buffer, owner_x, owner_z, &mut rand);
        let origin = (owner_x * CHUNK_WIDTH as i32, owner_z * CHUNK_WIDTH as i32);
        let biome = self.climate_at(origin.0 + 16, origin.1 + 16).2;
        let feature_value = self.feature_noise_at(origin.0 as f64 * 0.5, origin.1 as f64 * 0.5);
        tree::populate_from(buffer, owner_x, owner_z, biome, feature_value, &self.block_ids, &mut rand);
        plant::populate_from(self, buffer, owner_x, owner_z, &mut rand);
        spring::populate_from(buffer, owner_x, owner_z, &self.block_ids, &mut rand);
        snow::populate_from(buffer, owner_x, owner_z, &self.block_ids, |x, z| self.climate_at(x, z).0);
    }
}
