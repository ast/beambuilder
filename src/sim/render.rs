use crate::GameState;
use crate::sim::dynamic::DynamicState;
use crate::sim::fem::Stresses;
use crate::sim::graph::{NodeKind, TrussGraph};
use crate::sim::physics::YIELD_STRESS;
use bevy::prelude::*;

const BEAM_OK_COLOR: Color = Color::srgb(0.45, 1.0, 0.45);
const FREE_NODE_COLOR: Color = Color::srgb(0.85, 1.0, 0.85);
const ENGINE_COLOR: Color = Color::srgb(1.0, 0.55, 0.35);
const WAGON_COLOR: Color = Color::srgb(1.0, 0.85, 0.35);
const WHEEL_COLOR: Color = Color::srgb(0.85, 0.85, 0.85);
const COUPLER_COLOR: Color = Color::srgb(0.85, 0.6, 0.25);

const NODE_OUTER_R: f32 = 4.0;
const NODE_INNER_R: f32 = 1.5;

// Train car geometry (all measured from the car's simulation center).
// The collision circle (CAR_RADIUS = 20) extends 20 units below the center,
// which is exactly where the wheel bottoms sit, so the car rests cleanly on
// the deck without visual penetration.
const BODY_HALF_W: f32 = 22.0;
const BODY_HALF_H: f32 = 10.0;
const BODY_OFFSET_Y: f32 = 4.0;
const WHEEL_R: f32 = 6.0;
const WHEEL_OFFSET_X: f32 = 14.0;
const WHEEL_OFFSET_Y: f32 = -14.0;
const STACK_HALF_W: f32 = 4.0;
const STACK_HALF_H: f32 = 8.0;
const STACK_OFFSET_X: f32 = -12.0;
const STACK_OFFSET_Y: f32 = BODY_OFFSET_Y + BODY_HALF_H + STACK_HALF_H;

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

    if let Some(train) = &state.train {
        // Couplers between adjacent cars (at body level, not car center).
        for pair in train.cars.windows(2) {
            let p0 = pair[0].pos + Vec2::new(0.0, BODY_OFFSET_Y);
            let p1 = pair[1].pos + Vec2::new(0.0, BODY_OFFSET_Y);
            gizmos.line_2d(p0, p1, COUPLER_COLOR);
        }
        for (i, car) in train.cars.iter().enumerate() {
            let is_engine = i == 0;
            draw_train_car(&mut gizmos, car.pos, is_engine);
        }
    }
}

fn draw_train_car(gizmos: &mut Gizmos, pos: Vec2, is_engine: bool) {
    let body_color = if is_engine { ENGINE_COLOR } else { WAGON_COLOR };

    // Body.
    gizmos.rect_2d(
        Isometry2d::from_translation(pos + Vec2::new(0.0, BODY_OFFSET_Y)),
        Vec2::new(BODY_HALF_W * 2.0, BODY_HALF_H * 2.0),
        body_color,
    );

    // Wheels.
    for wx in [-WHEEL_OFFSET_X, WHEEL_OFFSET_X] {
        let iso = Isometry2d::from_translation(pos + Vec2::new(wx, WHEEL_OFFSET_Y));
        gizmos.circle_2d(iso, WHEEL_R, WHEEL_COLOR);
    }

    // Engine extras: smokestack.
    if is_engine {
        gizmos.rect_2d(
            Isometry2d::from_translation(pos + Vec2::new(STACK_OFFSET_X, STACK_OFFSET_Y)),
            Vec2::new(STACK_HALF_W * 2.0, STACK_HALF_H * 2.0),
            body_color,
        );
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
