use chorus::level::chunk::Chunk;
use chorus::level::generator::dimension::Generator;
use chorus::level::generator::phase::{Phase, PhaseInputs, Requirement, requirement, same_cell, self_requirement};
use chorus::level::generator::pos::ChunkPos;
use std::sync::Arc;

use super::NetherGenerator;
use crate::level::generator::shared::TerrainSource;
use crate::level::generator::shared::chunk_buffer::ChunkBuffer;
use crate::level::generator::shared::phases::{Population, assemble_chunk, backward_neighbors, forward_quad, owner_quad};
use crate::level::generator::shared::population::{self, PopulationSource};
use crate::level::generator::shared::quad_chunk_buffer::QuadChunkBuffer;
use crate::phase_value_enum;

struct NetherPhaseView<'a, 'b> {
    generator: &'a NetherGenerator,
    inputs: &'a PhaseInputs<'b, NetherGenerator>,
}

impl TerrainSource for NetherPhaseView<'_, '_> {
    fn raw_terrain(&self, x: i32, z: i32) -> ChunkBuffer {
        self.generator.raw_terrain(x, z)
    }

    fn carve_caves(&self, x: i32, z: i32, column: &mut ChunkBuffer) {
        self.generator.carve_caves(x, z, column);
    }

    fn terrain_and_caves(&self, x: i32, z: i32) -> Arc<ChunkBuffer> {
        self.inputs
            .try_get::<CavesPhase>(ChunkPos::new(x, z))
            .unwrap_or_else(|| panic!("terrain_and_caves({x}, {z}) missed the CavesPhase dependency cache"))
    }

    fn min_sub_chunk_y(&self) -> i8 {
        self.generator.min_sub_chunk_y()
    }

    fn dimension_sub_chunk_count(&self) -> usize {
        self.generator.dimension_sub_chunk_count()
    }

    fn air_id(&self) -> i32 {
        self.generator.air_id()
    }

    fn biome(&self) -> i32 {
        self.generator.biome()
    }
}

impl PopulationSource for NetherPhaseView<'_, '_> {
    fn owner_population(&self, owner_x: i32, owner_z: i32) -> Option<Arc<QuadChunkBuffer>> {
        self.inputs.try_get::<PopulationPhase>(ChunkPos::new(owner_x, owner_z))
    }

    fn run_population(&self, owner_x: i32, owner_z: i32, buffer: &mut QuadChunkBuffer) {
        self.generator.run_population_raw(owner_x, owner_z, buffer);
    }
}

pub struct TerrainPhase;

impl Phase<NetherGenerator> for TerrainPhase {
    type Output = ChunkBuffer;

    fn run(generator: &NetherGenerator, cell: ChunkPos, _inputs: &PhaseInputs<NetherGenerator>) -> Self::Output {
        generator.raw_terrain(cell.x, cell.z)
    }
}

pub struct CavesPhase;

impl Phase<NetherGenerator> for CavesPhase {
    type Output = ChunkBuffer;

    fn requires() -> Vec<Requirement<NetherGenerator>> {
        vec![requirement::<NetherGenerator, TerrainPhase>(same_cell)]
    }

    fn run(generator: &NetherGenerator, cell: ChunkPos, inputs: &PhaseInputs<NetherGenerator>) -> Self::Output {
        let mut column = (*inputs.get::<TerrainPhase>(cell)).clone();
        generator.carve_caves(cell.x, cell.z, &mut column);
        column
    }
}

pub struct PopulationPhase;

impl Phase<NetherGenerator> for PopulationPhase {
    type Output = QuadChunkBuffer;

    fn requires() -> Vec<Requirement<NetherGenerator>> {
        vec![
            requirement::<NetherGenerator, CavesPhase>(forward_quad),
            self_requirement::<NetherGenerator, PopulationPhase>(backward_neighbors, 1),
        ]
    }

    fn run(generator: &NetherGenerator, cell: ChunkPos, inputs: &PhaseInputs<NetherGenerator>) -> Self::Output {
        let view = NetherPhaseView { generator, inputs };
        population::populate_owner(&view, cell.x, cell.z)
    }
}

pub struct ColumnPhase;

impl Phase<NetherGenerator> for ColumnPhase {
    type Output = ChunkBuffer;

    fn requires() -> Vec<Requirement<NetherGenerator>> {
        vec![requirement::<NetherGenerator, CavesPhase>(same_cell), requirement::<NetherGenerator, PopulationPhase>(owner_quad)]
    }

    fn run(generator: &NetherGenerator, cell: ChunkPos, inputs: &PhaseInputs<NetherGenerator>) -> Self::Output {
        let view = NetherPhaseView { generator, inputs };
        let mut column = (*inputs.get::<CavesPhase>(cell)).clone();
        population::populate(&view, cell.x, cell.z, &mut column);
        column
    }
}

pub struct ChunkPhase;

impl Phase<NetherGenerator> for ChunkPhase {
    type Output = Chunk;

    fn requires() -> Vec<Requirement<NetherGenerator>> {
        vec![requirement::<NetherGenerator, ColumnPhase>(same_cell)]
    }

    fn run(generator: &NetherGenerator, cell: ChunkPos, inputs: &PhaseInputs<NetherGenerator>) -> Self::Output {
        let column = inputs.get::<ColumnPhase>(cell).as_ref().clone();
        assemble_chunk(generator, cell, column)
    }
}

phase_value_enum! {
    pub enum NetherPhaseValue for NetherGenerator {
        Terrain(TerrainPhase) => ChunkBuffer,
        Caves(CavesPhase) => ChunkBuffer,
        Population(PopulationPhase) => QuadChunkBuffer,
        Column(ColumnPhase) => ChunkBuffer,
        Chunk(ChunkPhase) => Chunk,
    }
}

impl Generator for NetherGenerator {
    type Terminal = ChunkPhase;
    type Value = NetherPhaseValue;
}
