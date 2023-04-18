use bevy::prelude::*;
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use kluring::KluringPlugin;

mod kluring;

fn main() {
    App::new()
    .add_plugins(DefaultPlugins)

    // remove in release
    // .add_plugin(WorldInspectorPlugin::new())

    .add_plugin(KluringPlugin)

    .run();
}
