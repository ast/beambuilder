pub mod delete;
pub mod draw;
pub mod grid;
pub mod history;
pub mod snap;

use bevy::prelude::*;

pub struct EditPlugin;

impl Plugin for EditPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            grid::GridPlugin,
            history::HistoryPlugin,
            draw::DrawPlugin,
            delete::DeletePlugin,
        ));
    }
}
