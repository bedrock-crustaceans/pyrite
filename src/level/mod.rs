use crate::level::generator::nether::NetherGenerator;
use crate::level::generator::overworld::OverworldGenerator;
use crate::level::generator::seed::parse_seed;
use bevy_ecs::prelude::Res;
use bevy_ecs::system::ResMut;
use chorus::level::dimension_type::DimensionType;
use chorus::level::generator::dimension::Dimension;
use chorus::level::level::Level;
use chorus::registry::block_registry::BlockRegistry;
use tracing::info;

pub mod generator;

pub fn insert_level(mut level: ResMut<Level>, registry: Res<BlockRegistry>) {
    // let seed_str = "2151901553968352745"; // title-screen seed
    // let seed_str = "3257840388504953787"; // pack.png seed
    // let seed_str = "Glacier";
    let seed_str = "gargamel";

    let seed = parse_seed(seed_str);

    let overworld = OverworldGenerator::new(seed, &registry);
    level.spawn = overworld.find_spawn();
    level.dimensions.insert(0, Dimension::new(DimensionType::Overworld, overworld));
    level.dimensions.insert(1, Dimension::new(DimensionType::Nether, NetherGenerator::new(seed, &registry)));

    info!("registered b1.7.3 level with seed \"{}\" ({}), spawn at {}", seed_str, seed, level.spawn);
}
