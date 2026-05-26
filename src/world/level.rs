use crate::cli::LevelPath;
use bevy::prelude::*;
use bevy_common_assets::ron::RonAssetPlugin;
use serde::Deserialize;

/// Hand-authored level loaded from `assets/levels/*.level.ron`.
///
/// Coordinates are in world units (the camera sees them 1:1 in pixels at zoom = 1.0).
/// Points are stored as `[x, y]` tuples for trivial serde compatibility without
/// pulling in Bevy's `serialize` feature.
#[derive(Asset, TypePath, Debug, Deserialize)]
pub struct Level {
    /// Fixed support points the player can build from.
    pub anchors: Vec<[f32; 2]>,
    /// Polyline describing the terrain silhouette, left → right.
    pub terrain: Vec<[f32; 2]>,
    /// Where the test vehicle (engine car) starts.
    pub vehicle_spawn: [f32; 2],
    /// Goal position — vehicle reaches here to win.
    pub goal: [f32; 2],
}

impl Level {
    pub fn anchors_vec2(&self) -> impl Iterator<Item = Vec2> + '_ {
        self.anchors.iter().map(|p| Vec2::new(p[0], p[1]))
    }

    pub fn terrain_vec2(&self) -> impl Iterator<Item = Vec2> + '_ {
        self.terrain.iter().map(|p| Vec2::new(p[0], p[1]))
    }

    pub fn vehicle_spawn_vec2(&self) -> Vec2 {
        Vec2::new(self.vehicle_spawn[0], self.vehicle_spawn[1])
    }

    pub fn goal_vec2(&self) -> Vec2 {
        Vec2::new(self.goal[0], self.goal[1])
    }
}

/// Handle to the currently active level. Inserted at startup.
#[derive(Resource)]
pub struct CurrentLevel(pub Handle<Level>);

pub struct LevelPlugin;

impl Plugin for LevelPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(RonAssetPlugin::<Level>::new(&["level.ron"]))
            .add_systems(Startup, load_initial_level);
    }
}

fn load_initial_level(
    mut commands: Commands,
    assets: Res<AssetServer>,
    level_path: Res<LevelPath>,
) {
    let handle = assets.load::<Level>(level_path.0.clone());
    commands.insert_resource(CurrentLevel(handle));
}
