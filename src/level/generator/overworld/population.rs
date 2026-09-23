use crate::level::generator::java_rand::JavaRand;

use super::column::Column;
use super::owner_buffer::OwnerBuffer;
use super::{OverworldGenerator, dungeon, lake, owner_chunk_seed, plant, spring, tree, vein};

/// Builds owner `(owner_x, owner_z)`'s population, seeding its home chunk from its 3
/// backward neighbors' isolated contributions first - a feature's gating can depend on
/// what a backward neighbor already carved/placed there (confirmed against mc173).
pub(super) fn populate_owner(generator: &OverworldGenerator, owner_x: i32, owner_z: i32) -> OwnerBuffer {
    let mut buffer = OwnerBuffer::new(generator, owner_x, owner_z);
    buffer.set_column(owner_x, owner_z, backward_seed(generator, owner_x, owner_z));
    run_population(generator, owner_x, owner_z, &mut buffer);
    buffer
}

/// Same as `populate_owner`, but seeds its home chunk from pure `terrain_and_caves`
/// instead of backward neighbors - used only as the backward-neighbor source itself,
/// so that correction stays bounded to one hop instead of recursing indefinitely.
pub(super) fn populate_owner_isolated(generator: &OverworldGenerator, owner_x: i32, owner_z: i32) -> OwnerBuffer {
    let mut buffer = OwnerBuffer::new(generator, owner_x, owner_z);
    run_population(generator, owner_x, owner_z, &mut buffer);
    buffer
}

/// Lakes, dungeons, veins, trees, plants, then springs - one continuous RNG stream in
/// the reference's own order; reordering any of these desyncs everything after it.
fn run_population(generator: &OverworldGenerator, owner_x: i32, owner_z: i32, buffer: &mut OwnerBuffer) {
    let mut rand = JavaRand::new(owner_chunk_seed(generator.seed, owner_x, owner_z));

    lake::populate_from(generator, buffer, owner_x, owner_z, &mut rand);
    dungeon::populate_from(generator, buffer, owner_x, owner_z, &mut rand);
    vein::populate_from(generator, buffer, owner_x, owner_z, &mut rand);
    tree::populate_from(generator, buffer, owner_x, owner_z, &mut rand);
    plant::populate_from(generator, buffer, owner_x, owner_z, &mut rand);
    spring::populate_from(generator, buffer, owner_x, owner_z, &mut rand);
}

fn backward_seed(generator: &OverworldGenerator, owner_x: i32, owner_z: i32) -> Column {
    let baseline = generator.terrain_and_caves(owner_x, owner_z);
    let mut column = (*baseline).clone();

    for (backward_x, backward_z) in [(owner_x - 1, owner_z - 1), (owner_x - 1, owner_z), (owner_x, owner_z - 1)] {
        let backward_buffer = generator.owner_population_isolated(backward_x, backward_z);
        overlay_owner(&baseline, &backward_buffer, owner_x, owner_z, &mut column);
    }

    column
}

/// Copies `owner_buffer`'s slice at `(x, z)` onto `column` wherever it differs from
/// `baseline` - a no-op if `owner_buffer` never touched `(x, z)`.
fn overlay_owner(baseline: &Column, owner_buffer: &OwnerBuffer, x: i32, z: i32, column: &mut Column) {
    let Some(owner_column) = owner_buffer.column(x, z) else {
        return;
    };

    for lx in 0..super::CHUNK_WIDTH {
        for ly in 0..super::CHUNK_HEIGHT {
            for lz in 0..super::CHUNK_WIDTH {
                let owner_value = owner_column.get(lx, ly, lz);
                if owner_value != baseline.get(lx, ly, lz) {
                    column.set(lx, ly, lz, owner_value);
                }
            }
        }
    }
}

/// Assembles chunk `(x, z)`'s populated column from the 4 owners that can legitimately
/// place something here (`{x-1,x} x {z-1,z}`), applied in raster order.
pub fn populate(generator: &OverworldGenerator, x: i32, z: i32, column: &mut Column) {
    let baseline = generator.terrain_and_caves(x, z);

    for owner_x in x - 1..=x {
        for owner_z in z - 1..=z {
            let owner_buffer = generator.owner_population(owner_x, owner_z);
            overlay_owner(&baseline, &owner_buffer, x, z, column);
        }
    }
}
