use chorus::level::biome::biome_id::BiomeID;
use chorus::level::chunk::Chunk;
use chorus::level::dimension_type::DimensionType;
use chorus::level::generator::dimension::Generator;
use chorus::level::generator::error::PhaseError;
use chorus::level::generator::phase::{Phase, PhaseInputs, Requirement, SAME_CELL};
use chorus::level::generator::pos::ChunkPos;

use super::{BiomeGrid, SkyGenerator};
use crate::level::generator::shared::chunk_buffer::ChunkBuffer;
use crate::level::generator::shared::quad_chunk_buffer::{QuadChanges, QuadChunkBuffer};

const FORWARD_QUAD: &[(i32, i32)] = &[(0, 0), (1, 0), (0, 1), (1, 1)];
const OWNER_QUAD: &[(i32, i32)] = &[(-1, -1), (-1, 0), (0, -1), (0, 0)];
const BACKWARD_NEIGHBORS: &[(i32, i32)] = &[(-1, -1), (-1, 0), (0, -1)];

impl Generator for SkyGenerator {
    type Terminal = ChunkPhase;
}

pub struct BiomePhase;

impl Phase<SkyGenerator> for BiomePhase {
    type Output = BiomeGrid;

    fn run(generator: &SkyGenerator, cell: ChunkPos, _inputs: &mut PhaseInputs<SkyGenerator>) -> Result<BiomeGrid, PhaseError> {
        Ok(generator.biomes(cell.x, cell.z))
    }
}

pub struct TerrainPhase;

impl Phase<SkyGenerator> for TerrainPhase {
    type Output = ChunkBuffer;

    fn run(generator: &SkyGenerator, cell: ChunkPos, _inputs: &mut PhaseInputs<SkyGenerator>) -> Result<ChunkBuffer, PhaseError> {
        Ok(generator.terrain(cell.x, cell.z))
    }
}

pub struct SurfacePhase;

impl Phase<SkyGenerator> for SurfacePhase {
    type Output = ChunkBuffer;

    fn requires() -> Vec<Requirement<SkyGenerator>> {
        vec![TerrainPhase::at(SAME_CELL), BiomePhase::at(SAME_CELL)]
    }

    fn run(generator: &SkyGenerator, cell: ChunkPos, inputs: &mut PhaseInputs<SkyGenerator>) -> Result<ChunkBuffer, PhaseError> {
        let mut column = inputs.take::<TerrainPhase>(cell)?;
        let biomes = inputs.get::<BiomePhase>(cell)?;
        generator.surface(cell.x, cell.z, &mut column, &biomes);
        Ok(column)
    }
}

pub struct CavesPhase;

impl Phase<SkyGenerator> for CavesPhase {
    type Output = ChunkBuffer;

    const RETAIN: usize = 1024;

    fn requires() -> Vec<Requirement<SkyGenerator>> {
        vec![SurfacePhase::at(SAME_CELL)]
    }

    fn run(generator: &SkyGenerator, cell: ChunkPos, inputs: &mut PhaseInputs<SkyGenerator>) -> Result<ChunkBuffer, PhaseError> {
        let mut column = inputs.take::<SurfacePhase>(cell)?;
        generator.carve_caves(cell.x, cell.z, &mut column);
        Ok(column)
    }
}

pub struct PopulationPhase;

impl Phase<SkyGenerator> for PopulationPhase {
    type Output = QuadChanges;

    const RETAIN: usize = 1024;

    fn requires() -> Vec<Requirement<SkyGenerator>> {
        vec![CavesPhase::at(FORWARD_QUAD), PopulationPhase::at(BACKWARD_NEIGHBORS).max_hops(1)]
    }

    fn run(generator: &SkyGenerator, cell: ChunkPos, inputs: &mut PhaseInputs<SkyGenerator>) -> Result<QuadChanges, PhaseError> {
        let caves = |dx: i32, dz: i32| inputs.get::<CavesPhase>(ChunkPos::new(cell.x + dx, cell.z + dz));

        let baseline = caves(0, 0)?;
        let mut home = (*baseline).clone();
        for &(dx, dz) in BACKWARD_NEIGHBORS {
            if let Some(neighbor) = inputs.try_get::<PopulationPhase>(ChunkPos::new(cell.x + dx, cell.z + dz)) {
                neighbor.overlay_onto(cell.x, cell.z, &baseline, &mut home);
            }
        }

        let columns = [[home, (*caves(0, 1)?).clone()], [(*caves(1, 0)?).clone(), (*caves(1, 1)?).clone()]];
        let mut quad = QuadChunkBuffer::new(cell.x, cell.z, generator.block_ids.air, columns);
        generator.populate(cell.x, cell.z, &mut quad);
        quad.into_changes().map_err(PhaseError::custom)
    }
}

pub struct ColumnPhase;

impl Phase<SkyGenerator> for ColumnPhase {
    type Output = ChunkBuffer;

    fn requires() -> Vec<Requirement<SkyGenerator>> {
        vec![CavesPhase::at(SAME_CELL), PopulationPhase::at(OWNER_QUAD)]
    }

    fn run(_generator: &SkyGenerator, cell: ChunkPos, inputs: &mut PhaseInputs<SkyGenerator>) -> Result<ChunkBuffer, PhaseError> {
        let baseline = inputs.get::<CavesPhase>(cell)?;
        let mut column = (*baseline).clone();
        for &(dx, dz) in OWNER_QUAD {
            let owner = inputs.get::<PopulationPhase>(ChunkPos::new(cell.x + dx, cell.z + dz))?;
            owner.overlay_onto(cell.x, cell.z, &baseline, &mut column);
        }
        Ok(column)
    }
}

pub struct ChunkPhase;

impl Phase<SkyGenerator> for ChunkPhase {
    type Output = Chunk;

    fn requires() -> Vec<Requirement<SkyGenerator>> {
        vec![ColumnPhase::at(SAME_CELL)]
    }

    fn run(generator: &SkyGenerator, cell: ChunkPos, inputs: &mut PhaseInputs<SkyGenerator>) -> Result<Chunk, PhaseError> {
        Ok(inputs.take::<ColumnPhase>(cell)?.into_chunk(cell, DimensionType::Overworld, generator.block_ids.air, BiomeID::PLAINS))
    }
}
