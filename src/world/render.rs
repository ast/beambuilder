use crate::world::level::{CurrentLevel, Level};
use bevy::prelude::*;

const TERRAIN_COLOR: Color = Color::srgb(0.55, 0.85, 0.75);
const ANCHOR_COLOR: Color = Color::srgb(0.55, 1.0, 0.55);
const GOAL_COLOR: Color = Color::srgb(1.0, 0.85, 0.35);

const ANCHOR_OUTER_R: f32 = 8.0;
const ANCHOR_INNER_R: f32 = 4.0;
const GOAL_HALF: f32 = 12.0;

pub struct WorldRenderPlugin;

impl Plugin for WorldRenderPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, draw_level);
    }
}

fn draw_level(
    mut gizmos: Gizmos,
    current: Option<Res<CurrentLevel>>,
    levels: Res<Assets<Level>>,
) {
    let Some(handle) = current.as_ref().map(|c| &c.0) else {
        return;
    };
    let Some(level) = levels.get(handle) else {
        return;
    };

    let terrain: Vec<Vec2> = level.terrain_vec2().collect();
    for pair in terrain.windows(2) {
        gizmos.line_2d(pair[0], pair[1], TERRAIN_COLOR);
    }

    for a in level.anchors_vec2() {
        let iso = Isometry2d::from_translation(a);
        gizmos.circle_2d(iso, ANCHOR_OUTER_R, ANCHOR_COLOR);
        gizmos.circle_2d(iso, ANCHOR_INNER_R, ANCHOR_COLOR);
    }

    let g = level.goal_vec2();
    gizmos.line_2d(
        g + Vec2::new(-GOAL_HALF, -GOAL_HALF),
        g + Vec2::new(GOAL_HALF, GOAL_HALF),
        GOAL_COLOR,
    );
    gizmos.line_2d(
        g + Vec2::new(-GOAL_HALF, GOAL_HALF),
        g + Vec2::new(GOAL_HALF, -GOAL_HALF),
        GOAL_COLOR,
    );
}
