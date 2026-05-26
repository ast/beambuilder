pub mod dynamic;
pub mod fem;
pub mod graph;
pub mod physics;
pub mod render;

use bevy::prelude::*;

pub struct SimPlugin;

impl Plugin for SimPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            graph::GraphPlugin,
            fem::FemPlugin,
            dynamic::DynamicPlugin,
            render::SimRenderPlugin,
        ));
    }
}
