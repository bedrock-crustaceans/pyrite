use bevy_app::Startup;
use bevy_ecs::prelude::IntoScheduleConfigs;
use chorus::Chorus;
use chorus::registry::init_level;
use pyrite::level::insert_level;

fn main() {
    let mut app = Chorus::init();
    app.add_systems(Startup, insert_level.after(init_level));
    app.run();
}
