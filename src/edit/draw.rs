use crate::GameState;
use crate::edit::history::{History, Op};
use crate::edit::snap::{SnapResult, snap, world_cursor};
use crate::sim::graph::{Beam, MAX_BEAM_LENGTH, Node, NodeKind, TrussGraph};
use bevy::prelude::*;

const PREVIEW_COLOR: Color = Color::srgb(0.45, 1.0, 0.45);
const PREVIEW_INVALID: Color = Color::srgb(1.0, 0.45, 0.45);

#[derive(Resource, Default)]
struct DragState {
    start: Option<SnapResult>,
}

pub struct DrawPlugin;

impl Plugin for DrawPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DragState>().add_systems(
            Update,
            (begin_drag, update_drag_preview, commit_drag)
                .chain()
                .run_if(in_state(GameState::Edit)),
        );
    }
}

fn begin_drag(
    buttons: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    cameras: Query<(&Camera, &GlobalTransform)>,
    graph: Res<TrussGraph>,
    mut drag: ResMut<DragState>,
) {
    if !buttons.just_pressed(MouseButton::Left) {
        return;
    }
    let Some(world) = world_cursor(&windows, &cameras) else {
        return;
    };
    drag.start = Some(snap(world, &graph));
}

fn update_drag_preview(
    buttons: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    cameras: Query<(&Camera, &GlobalTransform)>,
    graph: Res<TrussGraph>,
    drag: Res<DragState>,
    mut gizmos: Gizmos,
) {
    let Some(start) = drag.start else { return };
    if !buttons.pressed(MouseButton::Left) {
        return;
    }
    let Some(world) = world_cursor(&windows, &cameras) else {
        return;
    };
    let end = snap(world, &graph);
    let length = (end.pos() - start.pos()).length();
    let color = if start.pos() == end.pos() || length > MAX_BEAM_LENGTH {
        PREVIEW_INVALID
    } else {
        PREVIEW_COLOR
    };
    gizmos.line_2d(start.pos(), end.pos(), color);
}

fn commit_drag(
    buttons: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    cameras: Query<(&Camera, &GlobalTransform)>,
    mut graph: ResMut<TrussGraph>,
    mut drag: ResMut<DragState>,
    mut history: ResMut<History>,
) {
    if !buttons.just_released(MouseButton::Left) {
        return;
    }
    let Some(start) = drag.start.take() else {
        return;
    };
    let Some(world) = world_cursor(&windows, &cameras) else {
        return;
    };
    let end = snap(world, &graph);
    if start.pos() == end.pos() {
        return;
    }
    if (end.pos() - start.pos()).length() > MAX_BEAM_LENGTH {
        return;
    }

    let mut created_nodes = Vec::with_capacity(2);
    let a = match start {
        SnapResult::Existing(id, _) => id,
        SnapResult::Grid(p) => {
            let id = graph.add_node(p, NodeKind::Free);
            created_nodes.push((id, Node { pos: p, kind: NodeKind::Free }));
            id
        }
    };
    let b = match end {
        SnapResult::Existing(id, _) => id,
        SnapResult::Grid(p) => {
            let id = graph.add_node(p, NodeKind::Free);
            created_nodes.push((id, Node { pos: p, kind: NodeKind::Free }));
            id
        }
    };

    match graph.add_beam(a, b) {
        Some(beam_id) => {
            history.push(Op::AddBeam {
                beam_id,
                beam: Beam { a, b },
                created_nodes,
            });
        }
        None => {
            // Rejected (duplicate or self-loop); roll back any nodes we created.
            for (id, _) in created_nodes {
                graph.remove_node(id);
            }
        }
    }
}
