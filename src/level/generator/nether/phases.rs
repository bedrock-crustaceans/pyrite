use chorus::worldgen::shared::biome::pack_ids::HELL;
use chorus::level::chunk::Chunk;
use chorus::level::dimension_type::DimensionType;
use chorus::level::generator::dimension::Generator;
use chorus::level::generator::error::PhaseError;
use chorus::level::generator::phase::{Dependency, Phase, PhaseContext, SAME_CELL};
use glam::IVec2;

use super::NetherGenerator;
use crate::level::generator::shared::chunk_buffer::ChunkBuffer;
use crate::level::generator::shared::quad_chunk_buffer::{QuadChanges, QuadChunkBuffer};

const FORWARD_QUAD: &[(i32, i32)] = &[(0, 0), (1, 0), (0, 1), (1, 1)];
const OWNER_QUAD: &[(i32, i32)] = &[(-1, -1), (-1, 0), (0, -1), (0, 0)];
const BACKWARD_NEIGHBORS: &[(i32, i32)] = &[(-1, -1), (-1, 0), (0, -1)];

impl Generator for NetherGenerator {
    type Terminal = ChunkPhase;
}

pub struct TerrainPhase;

impl Phase<NetherGenerator> for TerrainPhase {
    type Output = ChunkBuffer;

    fn run(generator: &NetherGenerator, cell: IVec2, _context: &mut PhaseContext<NetherGenerator>) -> Result<ChunkBuffer, PhaseError> {
        Ok(generator.terrain(cell.x, cell.y))
    }
}

pub struct SurfacePhase;

impl Phase<NetherGenerator> for SurfacePhase {
    type Output = ChunkBuffer;

    fn dependencies() -> Vec<Dependency<NetherGenerator>> {
        vec![Dependency::on::<TerrainPhase>(SAME_CELL)]
    }

    fn run(generator: &NetherGenerator, cell: IVec2, context: &mut PhaseContext<NetherGenerator>) -> Result<ChunkBuffer, PhaseError> {
        let mut column = context.take::<TerrainPhase>(cell)?;
        generator.surface(cell.x, cell.y, &mut column);
        Ok(column)
    }
}

pub struct CavesPhase;

impl Phase<NetherGenerator> for CavesPhase {
    type Output = ChunkBuffer;

    const RETAIN: usize = 1024;

    fn dependencies() -> Vec<Dependency<NetherGenerator>> {
        vec![Dependency::on::<SurfacePhase>(SAME_CELL)]
    }

    fn run(generator: &NetherGenerator, cell: IVec2, context: &mut PhaseContext<NetherGenerator>) -> Result<ChunkBuffer, PhaseError> {
        let mut column = context.take::<SurfacePhase>(cell)?;
        generator.carve_caves(cell.x, cell.y, &mut column);
        Ok(column)
    }
}

pub struct PopulationPhase;

impl Phase<NetherGenerator> for PopulationPhase {
    type Output = QuadChanges;

    const RETAIN: usize = 1024;

    fn dependencies() -> Vec<Dependency<NetherGenerator>> {
        vec![Dependency::on::<CavesPhase>(FORWARD_QUAD), Dependency::on::<PopulationPhase>(BACKWARD_NEIGHBORS).max_hops(1)]
    }

    fn run(generator: &NetherGenerator, cell: IVec2, context: &mut PhaseContext<NetherGenerator>) -> Result<QuadChanges, PhaseError> {
        let caves = |dx: i32, dz: i32| context.get::<CavesPhase>(IVec2::new(cell.x + dx, cell.y + dz));

        let baseline = caves(0, 0)?;
        let mut home = (*baseline).clone();
        for &(dx, dz) in BACKWARD_NEIGHBORS {
            if let Some(neighbor) = context.try_get::<PopulationPhase>(IVec2::new(cell.x + dx, cell.y + dz)) {
                neighbor.overlay_onto(cell.x, cell.y, &baseline, &mut home);
            }
        }

        let columns = [[home, (*caves(0, 1)?).clone()], [(*caves(1, 0)?).clone(), (*caves(1, 1)?).clone()]];
        let mut quad = QuadChunkBuffer::new(cell.x, cell.y, generator.block_ids.air, columns);
        generator.populate(cell.x, cell.y, &mut quad);
        quad.into_changes().map_err(PhaseError::custom)
    }
}

pub struct ColumnPhase;

impl Phase<NetherGenerator> for ColumnPhase {
    type Output = ChunkBuffer;

    fn dependencies() -> Vec<Dependency<NetherGenerator>> {
        vec![Dependency::on::<CavesPhase>(SAME_CELL), Dependency::on::<PopulationPhase>(OWNER_QUAD)]
    }

    fn run(_generator: &NetherGenerator, cell: IVec2, context: &mut PhaseContext<NetherGenerator>) -> Result<ChunkBuffer, PhaseError> {
        let baseline = context.get::<CavesPhase>(cell)?;
        let mut column = (*baseline).clone();
        for &(dx, dz) in OWNER_QUAD {
            let owner = context.get::<PopulationPhase>(IVec2::new(cell.x + dx, cell.y + dz))?;
            owner.overlay_onto(cell.x, cell.y, &baseline, &mut column);
        }
        Ok(column)
    }
}

pub struct ChunkPhase;

impl Phase<NetherGenerator> for ChunkPhase {
    type Output = Chunk;

    fn dependencies() -> Vec<Dependency<NetherGenerator>> {
        vec![Dependency::on::<ColumnPhase>(SAME_CELL)]
    }

    fn run(generator: &NetherGenerator, cell: IVec2, context: &mut PhaseContext<NetherGenerator>) -> Result<Chunk, PhaseError> {
        Ok(context.take::<ColumnPhase>(cell)?.into_chunk(cell, DimensionType::Nether, generator.block_ids.air, HELL))
    }
}
