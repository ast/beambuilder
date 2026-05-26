use crate::GameState;
use crate::sim::dynamic::DynamicState;
use crate::sim::fem::Stresses;
use crate::sim::graph::{NodeKind, TrussGraph};
use crate::sim::physics::YIELD_STRESS;
use bevy::prelude::*;

const BEAM_OK_COLOR: Color = Color::srgb(0.45, 1.0, 0.45);
const FREE_NODE_COLOR: Color = Color::srgb(0.85, 1.0, 0.85);
const VEHICLE_COLOR: Color = Color::srgb(1.0, 0.85, 0.35);

const NODE_OUTER_R: f32 = 4.0;
const NODE_INNER_R: f32 = 1.5;
const VEHICLE_INNER_R: f32 = 5.0;

pub struct SimRenderPlugin;

impl Plugin for SimRenderPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                draw_truss.run_if(in_state(GameState::Edit)),
                draw_dynamic.run_if(in_state(GameState::Test)),
            ),
        );
    }
}

fn draw_truss(mut gizmos: Gizmos, graph: Res<TrussGraph>, stresses: Res<Stresses>) {
    for (id, beam) in &graph.beams {
        let (Some(a), Some(b)) = (graph.node_pos(beam.a), graph.node_pos(beam.b)) else {
            continue;
        };
        let color = stresses
            .0
            .get(id)
            .map(|s| stress_color(*s))
            .unwrap_or(BEAM_OK_COLOR);
        gizmos.line_2d(a, b, color);
    }

    for node in graph.nodes.values() {
        if node.kind != NodeKind::Free {
            continue;
        }
        draw_dot(
            &mut gizmos,
            node.pos,
            FREE_NODE_COLOR,
            NODE_OUTER_R,
            NODE_INNER_R,
        );
    }
}

fn draw_dynamic(mut gizmos: Gizmos, state: Res<DynamicState>) {
    for beam in state.beams.values() {
        if beam.broken {
            continue;
        }
        let (Some(a), Some(b)) = (state.nodes.get(&beam.a), state.nodes.get(&beam.b)) else {
            continue;
        };
        gizmos.line_2d(a.pos, b.pos, BEAM_OK_COLOR);
    }

    for node in state.nodes.values() {
        if node.fixed {
            // Anchors are drawn by world::render
            continue;
        }
        draw_dot(
            &mut gizmos,
            node.pos,
            FREE_NODE_COLOR,
            NODE_OUTER_R,
            NODE_INNER_R,
        );
    }

    if let Some(v) = state.vehicle {
        let iso = Isometry2d::from_translation(v.pos);
        gizmos.circle_2d(iso, v.radius, VEHICLE_COLOR);
        gizmos.circle_2d(iso, VEHICLE_INNER_R, VEHICLE_COLOR);
    }
}

fn draw_dot(gizmos: &mut Gizmos, center: Vec2, color: Color, outer: f32, inner: f32) {
    let iso = Isometry2d::from_translation(center);
    gizmos.circle_2d(iso, outer, color);
    gizmos.circle_2d(iso, inner, color);
}

fn stress_color(stress: f32) -> Color {
    let t = (stress.abs() / YIELD_STRESS).clamp(0.0, 1.0);
    let (r, g) = if t < 0.5 {
        (t * 2.0, 1.0)
    } else {
        (1.0, (1.0 - t) * 2.0)
    };
    Color::srgb(r, g, 0.2)
}
