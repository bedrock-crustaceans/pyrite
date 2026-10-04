use bevy_app::Startup;
use bevy_ecs::prelude::IntoScheduleConfigs;
use chorus::Chorus;
use chorus::registry::LevelStartup;
use pyrite::level::override_level;

fn main() {
    let mut app = Chorus::init();
    app.add_systems(Startup, override_level.in_set(LevelStartup::Dimensions));
    app.run();
}
