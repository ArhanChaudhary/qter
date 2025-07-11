use bevy::{DefaultPlugins, app::App};

pub fn demo(robot: bool) {
    App::new().add_plugins(DefaultPlugins).run();
}
