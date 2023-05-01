use bevy::{prelude::*, window::PresentMode};
use kluring::KluringPlugin;
use bevy_inspector_egui::quick::WorldInspectorPlugin;

mod kluring;

fn main() {
    App::new()

    .add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            present_mode: PresentMode::AutoNoVsync, // Reduces input lag.
            fit_canvas_to_parent: true,
            ..default()
        }),
        ..default()
    }))

    // remove in release
    // .add_plugin(WorldInspectorPlugin::new())

    .add_plugin(KluringPlugin)

    .run();
}
