use crate::GameState;
use bevy::gizmos::config::{GizmoConfigGroup, GizmoConfigStore};
use bevy::prelude::*;

pub const GRID_SIZE: f32 = 32.0;

/// Every Nth line gets the "major" weight + brightness.
const MAJOR_EVERY: i32 = 4;
const GRID_EXTENT: f32 = 2000.0;

const MINOR_COLOR: Color = Color::srgba(0.20, 0.24, 0.22, 0.55);
const MAJOR_COLOR: Color = Color::srgba(0.45, 0.55, 0.50, 0.85);

const MINOR_WIDTH: f32 = 1.0;
const MAJOR_WIDTH: f32 = 1.8;

#[derive(Default, Reflect, GizmoConfigGroup)]
pub struct GridMinor;

#[derive(Default, Reflect, GizmoConfigGroup)]
pub struct GridMajor;

pub struct GridPlugin;

impl Plugin for GridPlugin {
    fn build(&self, app: &mut App) {
        app.init_gizmo_group::<GridMinor>()
            .init_gizmo_group::<GridMajor>()
            .add_systems(Startup, configure_grid_gizmos)
            .add_systems(Update, draw_grid.run_if(in_state(GameState::Edit)));
    }
}

fn configure_grid_gizmos(mut store: ResMut<GizmoConfigStore>) {
    store.config_mut::<GridMinor>().0.line.width = MINOR_WIDTH;
    store.config_mut::<GridMajor>().0.line.width = MAJOR_WIDTH;
}

#[allow(dead_code)] // used in M3
pub fn snap_to_grid(world_pos: Vec2) -> Vec2 {
    Vec2::new(
        (world_pos.x / GRID_SIZE).round() * GRID_SIZE,
        (world_pos.y / GRID_SIZE).round() * GRID_SIZE,
    )
}

fn draw_grid(mut minor: Gizmos<GridMinor>, mut major: Gizmos<GridMajor>) {
    let lines = (GRID_EXTENT / GRID_SIZE) as i32;
    for i in -lines..=lines {
        let p = i as f32 * GRID_SIZE;
        let v_start = Vec2::new(p, -GRID_EXTENT);
        let v_end = Vec2::new(p, GRID_EXTENT);
        let h_start = Vec2::new(-GRID_EXTENT, p);
        let h_end = Vec2::new(GRID_EXTENT, p);
        if i.rem_euclid(MAJOR_EVERY) == 0 {
            major.line_2d(v_start, v_end, MAJOR_COLOR);
            major.line_2d(h_start, h_end, MAJOR_COLOR);
        } else {
            minor.line_2d(v_start, v_end, MINOR_COLOR);
            minor.line_2d(h_start, h_end, MINOR_COLOR);
        }
    }
}
