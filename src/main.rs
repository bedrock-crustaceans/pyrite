use bevy_app::Startup;
use bevy_ecs::prelude::IntoScheduleConfigs;
use chorus::Chorus;
use chorus::level::level::Level;
use pyrite::level::insert_level;

fn main() {
    Chorus::init()
        .add_systems(Startup, insert_level.after(Level::init))
        .run();
}