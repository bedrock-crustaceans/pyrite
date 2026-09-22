use std::collections::HashMap;

use bevy_ecs::prelude::Res;
use bevy_ecs::system::Commands;
use chorus::level::dimension::Dimension;
use chorus::level::level::Level;
use chorus::registry::block_registry::BlockRegistry;
use tracing::info;
use crate::level::generator::overworld::OverworldGenerator;
use crate::level::generator::seed::parse_seed;

pub mod generator;

pub fn insert_level(mut commands: Commands, _registry: Res<BlockRegistry>) {
    // let seed_str = "2151901553968352745"; // title-screen seed
    // let seed_str = "3257840388504953787"; // pack.png seed
    let seed_str = "Glacier"; // another og seed

    let seed = parse_seed(seed_str);

    let generator = OverworldGenerator::new(seed);
    let overworld = Dimension::new(0, -4, 19, generator);

    let mut level = Level { dimensions: HashMap::new() };
    level.dimensions.insert(0, overworld);

    info!("registered b1.7.3 overworld level with seed \"{}\" ({})", seed_str, seed);

    commands.insert_resource(level);
}
