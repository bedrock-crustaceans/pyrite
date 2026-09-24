use crate::level::generator::overworld::OverworldGenerator;
use crate::level::generator::seed::parse_seed;
use bevy_ecs::prelude::Res;
use bevy_ecs::system::ResMut;
use chorus::level::dimension::Dimension;
use chorus::level::level::Level;
use chorus::registry::block_registry::BlockRegistry;
use tracing::info;

pub mod generator;

// TEMPORARY, for testing the sky generator: chorus has no dimension switching yet,
// so this registers SkyGenerator as dimension 0 (in place of OverworldGenerator) so
// it's reachable at all. Revert to OverworldGenerator before merging.
pub fn insert_level(mut level: ResMut<Level>, registry: Res<BlockRegistry>) {
    // let seed_str = "2151901553968352745"; // title-screen seed
    // let seed_str = "3257840388504953787"; // pack.png seed
    let seed_str = "Glacier"; // another og seed

    let seed = parse_seed(seed_str);

    let generator = OverworldGenerator::new(seed, &registry);
    // let generator = NetherGenerator::new(seed, &registry);
    // let generator = SkyGenerator::new(seed, &registry);

    // Keep -4..19 regardless of the generator's own actual content range - this is
    // the dimension's declared subchunk range for the Bedrock protocol, and the
    // client crashes if it doesn't match what the overworld dimension normally uses.
    let overworld = Dimension::new(0, -4, 19, generator);

    level.dimensions.insert(0, overworld);

    info!("registered b1.7.3 level with seed \"{}\" ({})", seed_str, seed);
}
