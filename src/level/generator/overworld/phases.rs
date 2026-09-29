use std::sync::Arc;

use chorus::level::chunk::Chunk;
use chorus::level::generator::dimension::Generator;
use chorus::level::generator::phase::{Phase, PhaseInputs, Requirement, requirement, same_cell, self_requirement};
use chorus::level::generator::pos::ChunkPos;

use super::OverworldGenerator;
use crate::level::generator::shared::TerrainSource;
use crate::level::generator::shared::chunk_buffer::ChunkBuffer;
use crate::level::generator::shared::phases::{Population, assemble_chunk, backward_neighbors, forward_quad, owner_quad};
use crate::level::generator::shared::population::{self, PopulationSource};
use crate::level::generator::shared::quad_chunk_buffer::QuadChunkBuffer;
use crate::phase_value_enum;

struct OverworldPhaseView<'a, 'b> {
    generator: &'a OverworldGenerator,
    inputs: &'a PhaseInputs<'b, OverworldGenerator>,
}

impl TerrainSource for OverworldPhaseView<'_, '_> {
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

impl PopulationSource for OverworldPhaseView<'_, '_> {
    fn owner_population(&self, owner_x: i32, owner_z: i32) -> Option<Arc<QuadChunkBuffer>> {
        self.inputs.try_get::<PopulationPhase>(ChunkPos::new(owner_x, owner_z))
    }

    fn run_population(&self, owner_x: i32, owner_z: i32, buffer: &mut QuadChunkBuffer) {
        self.generator.run_population_raw(owner_x, owner_z, buffer);
    }
}

pub struct TerrainPhase;

impl Phase<OverworldGenerator> for TerrainPhase {
    type Output = ChunkBuffer;

    fn run(generator: &OverworldGenerator, cell: ChunkPos, _inputs: &PhaseInputs<OverworldGenerator>) -> Self::Output {
        generator.raw_terrain(cell.x, cell.z)
    }
}

pub struct CavesPhase;

impl Phase<OverworldGenerator> for CavesPhase {
    type Output = ChunkBuffer;

    fn requires() -> Vec<Requirement<OverworldGenerator>> {
        vec![requirement::<OverworldGenerator, TerrainPhase>(same_cell)]
    }

    fn run(generator: &OverworldGenerator, cell: ChunkPos, inputs: &PhaseInputs<OverworldGenerator>) -> Self::Output {
        let mut column = (*inputs.get::<TerrainPhase>(cell)).clone();
        generator.carve_caves(cell.x, cell.z, &mut column);
        column
    }
}

pub struct PopulationPhase;

impl Phase<OverworldGenerator> for PopulationPhase {
    type Output = QuadChunkBuffer;

    fn requires() -> Vec<Requirement<OverworldGenerator>> {
        vec![
            requirement::<OverworldGenerator, CavesPhase>(forward_quad),
            self_requirement::<OverworldGenerator, PopulationPhase>(backward_neighbors, 1),
        ]
    }

    fn run(generator: &OverworldGenerator, cell: ChunkPos, inputs: &PhaseInputs<OverworldGenerator>) -> Self::Output {
        let view = OverworldPhaseView { generator, inputs };
        population::populate_owner(&view, cell.x, cell.z)
    }
}

pub struct ColumnPhase;

impl Phase<OverworldGenerator> for ColumnPhase {
    type Output = ChunkBuffer;

    fn requires() -> Vec<Requirement<OverworldGenerator>> {
        vec![requirement::<OverworldGenerator, CavesPhase>(same_cell), requirement::<OverworldGenerator, PopulationPhase>(owner_quad)]
    }

    fn run(generator: &OverworldGenerator, cell: ChunkPos, inputs: &PhaseInputs<OverworldGenerator>) -> Self::Output {
        let view = OverworldPhaseView { generator, inputs };
        let mut column = (*inputs.get::<CavesPhase>(cell)).clone();
        population::populate(&view, cell.x, cell.z, &mut column);
        column
    }
}

pub struct ChunkPhase;

impl Phase<OverworldGenerator> for ChunkPhase {
    type Output = Chunk;

    fn requires() -> Vec<Requirement<OverworldGenerator>> {
        vec![requirement::<OverworldGenerator, ColumnPhase>(same_cell)]
    }

    fn run(generator: &OverworldGenerator, cell: ChunkPos, inputs: &PhaseInputs<OverworldGenerator>) -> Self::Output {
        let column = inputs.get::<ColumnPhase>(cell).as_ref().clone();
        assemble_chunk(generator, cell, column)
    }
}

phase_value_enum! {
    pub enum OverworldPhaseValue for OverworldGenerator {
        Terrain(TerrainPhase) => ChunkBuffer,
        Caves(CavesPhase) => ChunkBuffer,
        Population(PopulationPhase) => QuadChunkBuffer,
        Column(ColumnPhase) => ChunkBuffer,
        Chunk(ChunkPhase) => Chunk,
    }
}

impl Generator for OverworldGenerator {
    type Terminal = ChunkPhase;
    type Value = OverworldPhaseValue;
}
