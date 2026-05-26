pub mod level;
pub mod render;

use bevy::prelude::*;

pub struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((level::LevelPlugin, render::WorldRenderPlugin));
    }
}
