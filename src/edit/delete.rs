use crate::GameState;
use crate::edit::history::{History, Op};
use crate::edit::snap::world_cursor;
use crate::sim::graph::{Beam, BeamId, NodeKind, TrussGraph};
use bevy::prelude::*;

/// World-unit pickup tolerance for right-click delete.
const DELETE_RADIUS: f32 = 10.0;

pub struct DeletePlugin;

impl Plugin for DeletePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            delete_under_cursor.run_if(in_state(GameState::Edit)),
        );
    }
}

fn delete_under_cursor(
    buttons: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    cameras: Query<(&Camera, &GlobalTransform)>,
    mut graph: ResMut<TrussGraph>,
    mut history: ResMut<History>,
) {
    if !buttons.just_pressed(MouseButton::Right) {
        return;
    }
    let Some(world) = world_cursor(&windows, &cameras) else {
        return;
    };
    let Some((beam_id, beam)) = pick_beam(&graph, world) else {
        return;
    };

    graph.remove_beam(beam_id);

    // Sweep orphan free nodes (the deleted beam's endpoints, if no other beams use them).
    let mut removed_nodes = Vec::new();
    for endpoint in [beam.a, beam.b] {
        if let Some(node) = graph.nodes.get(&endpoint).copied()
            && node.kind == NodeKind::Free
            && graph.beam_count_for(endpoint) == 0
        {
            graph.remove_node(endpoint);
            removed_nodes.push((endpoint, node));
        }
    }

    history.push(Op::DeleteBeam {
        beam_id,
        beam,
        removed_nodes,
    });
}

fn pick_beam(graph: &TrussGraph, world: Vec2) -> Option<(BeamId, Beam)> {
    let mut best: Option<(BeamId, Beam, f32)> = None;
    for (id, beam) in &graph.beams {
        let (Some(a), Some(b)) = (graph.node_pos(beam.a), graph.node_pos(beam.b)) else {
            continue;
        };
        let d = point_to_segment_distance(world, a, b);
        if d < DELETE_RADIUS && best.is_none_or(|(_, _, bd)| d < bd) {
            best = Some((*id, *beam, d));
        }
    }
    best.map(|(id, beam, _)| (id, beam))
}

fn point_to_segment_distance(p: Vec2, a: Vec2, b: Vec2) -> f32 {
    let ab = b - a;
    let len_sq = ab.length_squared();
    if len_sq < 1e-6 {
        return (p - a).length();
    }
    let t = ((p - a).dot(ab) / len_sq).clamp(0.0, 1.0);
    (p - (a + ab * t)).length()
}
