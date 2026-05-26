mod camera;
mod edit;
mod sim;
mod ui;
mod world;

use bevy::gizmos::config::{DefaultGizmoConfigGroup, GizmoConfigStore};
use bevy::prelude::*;

#[derive(States, Default, Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum GameState {
    Menu,
    #[default]
    Edit,
    Test,
    Result,
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Beambuilder".into(),
                resolution: (1280, 800).into(),
                ..default()
            }),
            ..default()
        }))
        .insert_resource(ClearColor(Color::srgb(0.07, 0.08, 0.10)))
        .init_state::<GameState>()
        .add_plugins((
            camera::CameraPlugin,
            edit::EditPlugin,
            sim::SimPlugin,
            ui::UiPlugin,
            world::WorldPlugin,
        ))
        .add_systems(Startup, configure_gizmos)
        .run();
}

fn configure_gizmos(mut store: ResMut<GizmoConfigStore>) {
    let (config, _) = store.config_mut::<DefaultGizmoConfigGroup>();
    config.line.width = 3.0;
}
