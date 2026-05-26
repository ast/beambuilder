use crate::world::level::{CurrentLevel, Level};
use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use bevy::sprite_render::ColorMaterial;

const TERRAIN_COLOR: Color = Color::srgb(0.55, 0.85, 0.75);
const ANCHOR_COLOR: Color = Color::srgb(0.55, 1.0, 0.55);
const GROUND_FILL: Color = Color::srgb(0.40, 0.42, 0.46);

const ANCHOR_OUTER_R: f32 = 8.0;
const ANCHOR_INNER_R: f32 = 4.0;

/// Y value used as the bottom edge of the ground polygon. Anything below the
/// terrain polyline gets filled down to here. Pick a value far enough below
/// any reasonable level coordinate to stay off-screen at sane zooms.
const GROUND_BOTTOM_Y: f32 = -1500.0;

/// Z value of the ground mesh — below default (0) so beam gizmos render on top.
const GROUND_Z: f32 = -10.0;

#[derive(Component)]
struct GroundMesh;

pub struct WorldRenderPlugin;

impl Plugin for WorldRenderPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (spawn_ground_mesh, draw_level));
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
}

/// Builds a solid filled mesh under the terrain polyline once the Level asset
/// becomes available. Each (terrain[i], terrain[i+1]) pair plus their projections
/// onto the baseline forms a quad, split into two triangles.
fn spawn_ground_mesh(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    current: Option<Res<CurrentLevel>>,
    levels: Res<Assets<Level>>,
    existing: Query<Entity, With<GroundMesh>>,
    mut spawned: Local<bool>,
) {
    if *spawned && !existing.is_empty() {
        return;
    }
    let Some(handle) = current.as_ref().map(|c| &c.0) else {
        return;
    };
    let Some(level) = levels.get(handle) else {
        return;
    };

    let terrain: Vec<Vec2> = level.terrain_vec2().collect();
    if terrain.len() < 2 {
        return;
    }

    let n = terrain.len();
    let mut positions: Vec<[f32; 3]> = Vec::with_capacity(2 * n);
    for p in &terrain {
        positions.push([p.x, p.y, 0.0]);
    }
    for p in &terrain {
        positions.push([p.x, GROUND_BOTTOM_Y, 0.0]);
    }

    let mut indices: Vec<u32> = Vec::with_capacity(6 * (n - 1));
    for i in 0..(n - 1) {
        let top_l = i as u32;
        let top_r = (i + 1) as u32;
        let bot_l = (n + i) as u32;
        let bot_r = (n + i + 1) as u32;
        indices.extend_from_slice(&[top_l, bot_l, top_r, top_r, bot_l, bot_r]);
    }

    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default());
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_indices(Indices::U32(indices));

    commands.spawn((
        GroundMesh,
        Mesh2d(meshes.add(mesh)),
        MeshMaterial2d(materials.add(ColorMaterial::from(GROUND_FILL))),
        Transform::from_xyz(0.0, 0.0, GROUND_Z),
    ));
    *spawned = true;
}
