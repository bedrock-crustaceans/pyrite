use crate::level::generator::nether::NetherGenerator;
use crate::level::generator::overworld::OverworldGenerator;
use crate::level::generator::seed::parse_seed;
use bevy_ecs::prelude::Res;
use bevy_ecs::system::ResMut;
use chorus::config::Config;
use chorus::level::dimension_type::DimensionType;
use chorus::level::level::Level;
use chorus::registry::block_registry::BlockRegistry;

pub mod generator;

pub fn override_level(mut level: ResMut<Level>, registry: Res<BlockRegistry>, config: Res<Config>) {
    if level.is_new() {
        level.seed = config.level.seed.parse_with(parse_seed);
    }

    let overworld = OverworldGenerator::new(level.seed, &registry);
    let nether = NetherGenerator::new(level.seed, &registry);

    if level.is_new() {
        level.spawn = overworld.find_spawn();
    }

    level.insert_dimension(DimensionType::Overworld, overworld);
    level.insert_dimension(DimensionType::Nether, nether);
}
