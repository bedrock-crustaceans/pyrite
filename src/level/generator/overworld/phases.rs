//! Overworld's six generation phases, wired up explicitly - no generic "phase marker" plumbing
//! shared with the other dimensions, so this dimension's whole pipeline - what each phase
//! requires, and what it actually does - is readable end to end in this one file.

use std::sync::Arc;

use chorus::level::chunk::Chunk;
use chorus::level::generator::dimension::Generator;
use chorus::level::generator::phase::{Phase, PhaseInputs, Requirement, requirement, same_cell};
use chorus::level::generator::pos::ChunkPos;

use super::OverworldGenerator;
use crate::level::generator::shared::TerrainSource;
use crate::level::generator::shared::chunk_buffer::ChunkBuffer;
use crate::level::generator::shared::phases::{Population, assemble_chunk, backward_neighbors, forward_quad, owner_quad};
use crate::level::generator::shared::population::{self, PopulationSource};
use crate::level::generator::shared::quad_chunk_buffer::QuadChunkBuffer;

/// Adapts `OverworldGenerator` plus its already-resolved phase dependencies back into the
/// `TerrainSource`/`PopulationSource` interface the existing population/decoration code expects.
/// Terrain reads go through the phase graph's cache (`CavesPhase`) whenever the cell was part of
/// the phase's declared footprint - which covers every owner-quad construction, the genuinely hot
/// path - and fall back to a fresh, uncached computation only for the rare read outside it
/// (decoration occasionally looks a few blocks past its own quad).
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
        self.inputs.try_get::<CavesPhase>(ChunkPos::new(x, z)).unwrap_or_else(|| self.generator.terrain_and_caves(x, z))
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
    fn owner_population(&self, owner_x: i32, owner_z: i32) -> Arc<QuadChunkBuffer> {
        self.inputs.get::<OwnerPopulationPhase>(ChunkPos::new(owner_x, owner_z))
    }

    fn owner_population_isolated(&self, owner_x: i32, owner_z: i32) -> Arc<QuadChunkBuffer> {
        self.inputs.get::<OwnerPopulationIsolatedPhase>(ChunkPos::new(owner_x, owner_z))
    }

    fn run_population(&self, owner_x: i32, owner_z: i32, buffer: &mut QuadChunkBuffer) {
        self.generator.run_population_raw(owner_x, owner_z, buffer);
    }
}

/// Terrain shape only - density fields, surface - no caves carved in yet.
pub struct TerrainPhase;

impl Phase<OverworldGenerator> for TerrainPhase {
    type Output = ChunkBuffer;

    fn run(generator: &OverworldGenerator, cell: ChunkPos, _inputs: &PhaseInputs<OverworldGenerator>) -> Self::Output {
        generator.raw_terrain(cell.x, cell.z)
    }
}

/// Terrain with caves carved into it - the actual expensive work, cached here since every
/// owner-quad construction below reads it.
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

/// An owner's forward-quad population pass, run without any backward-neighbor overlay applied
/// first - used only as an input to computing other owners' backward-seed baselines.
pub struct OwnerPopulationIsolatedPhase;

impl Phase<OverworldGenerator> for OwnerPopulationIsolatedPhase {
    type Output = QuadChunkBuffer;

    fn requires() -> Vec<Requirement<OverworldGenerator>> {
        vec![requirement::<OverworldGenerator, CavesPhase>(forward_quad)]
    }

    fn run(generator: &OverworldGenerator, cell: ChunkPos, inputs: &PhaseInputs<OverworldGenerator>) -> Self::Output {
        let view = OverworldPhaseView { generator, inputs };
        population::populate_owner_isolated(&view, cell.x, cell.z)
    }
}

/// An owner's real forward-quad population pass, with its three backward neighbors' isolated
/// passes overlaid first.
pub struct OwnerPopulationPhase;

impl Phase<OverworldGenerator> for OwnerPopulationPhase {
    type Output = QuadChunkBuffer;

    fn requires() -> Vec<Requirement<OverworldGenerator>> {
        vec![
            requirement::<OverworldGenerator, CavesPhase>(forward_quad),
            requirement::<OverworldGenerator, OwnerPopulationIsolatedPhase>(backward_neighbors),
        ]
    }

    fn run(generator: &OverworldGenerator, cell: ChunkPos, inputs: &PhaseInputs<OverworldGenerator>) -> Self::Output {
        let view = OverworldPhaseView { generator, inputs };
        population::populate_owner(&view, cell.x, cell.z)
    }
}

/// One column's final block data: terrain+caves with all four owners touching it (the
/// backward-looking `owner_quad`) overlaid on top.
pub struct ColumnPhase;

impl Phase<OverworldGenerator> for ColumnPhase {
    type Output = ChunkBuffer;

    fn requires() -> Vec<Requirement<OverworldGenerator>> {
        vec![
            requirement::<OverworldGenerator, CavesPhase>(same_cell),
            requirement::<OverworldGenerator, OwnerPopulationPhase>(owner_quad),
        ]
    }

    fn run(generator: &OverworldGenerator, cell: ChunkPos, inputs: &PhaseInputs<OverworldGenerator>) -> Self::Output {
        let view = OverworldPhaseView { generator, inputs };
        let mut column = (*inputs.get::<CavesPhase>(cell)).clone();
        population::populate(&view, cell.x, cell.z, &mut column);
        column
    }
}

/// The terminal phase: assembles a column's blocks into the Bedrock-protocol `Chunk` shape.
pub struct ChunkPhase;

impl Phase<OverworldGenerator> for ChunkPhase {
    type Output = Chunk;

    fn requires() -> Vec<Requirement<OverworldGenerator>> {
        vec![requirement::<OverworldGenerator, ColumnPhase>(same_cell)]
    }

    fn run(generator: &OverworldGenerator, cell: ChunkPos, inputs: &PhaseInputs<OverworldGenerator>) -> Self::Output {
        let column = inputs.get::<ColumnPhase>(cell);
        assemble_chunk(generator, cell, &column)
    }
}

impl Generator for OverworldGenerator {
    type Terminal = ChunkPhase;
}
