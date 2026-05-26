use crate::sim::graph::{Beam, BeamId, Node, NodeId, NodeKind, TrussGraph};
use bevy::prelude::*;
use std::collections::VecDeque;

const MAX_HISTORY: usize = 100;

/// A reversible edit operation. Storing the data on the way in means undo can
/// faithfully restore IDs and orphan-node cleanup behavior.
#[derive(Clone, Debug)]
pub enum Op {
    AddBeam {
        beam_id: BeamId,
        beam: Beam,
        /// Free nodes that were created as a side-effect (one or two of the beam's endpoints).
        created_nodes: Vec<(NodeId, Node)>,
    },
    DeleteBeam {
        beam_id: BeamId,
        beam: Beam,
        /// Free nodes that became orphaned and were also removed.
        removed_nodes: Vec<(NodeId, Node)>,
    },
    /// Wipe every beam and every Free node in one shot. Anchors stay put.
    ClearAll {
        beams: Vec<(BeamId, Beam)>,
        removed_nodes: Vec<(NodeId, Node)>,
    },
}

#[derive(Resource, Default)]
pub struct History {
    undo_stack: VecDeque<Op>,
    redo_stack: VecDeque<Op>,
}

impl History {
    pub fn push(&mut self, op: Op) {
        self.undo_stack.push_back(op);
        if self.undo_stack.len() > MAX_HISTORY {
            self.undo_stack.pop_front();
        }
        self.redo_stack.clear();
    }
}

pub struct HistoryPlugin;

impl Plugin for HistoryPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<History>()
            .add_systems(
                Update,
                (undo_redo_input, clear_all_input)
                    .run_if(in_state(crate::GameState::Edit)),
            );
    }
}

fn undo_redo_input(
    keys: Res<ButtonInput<KeyCode>>,
    mut history: ResMut<History>,
    mut graph: ResMut<TrussGraph>,
) {
    let ctrl = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
    let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    if !ctrl {
        return;
    }

    if keys.just_pressed(KeyCode::KeyZ)
        && !shift
        && let Some(op) = history.undo_stack.pop_back()
    {
        apply_inverse(&op, &mut graph);
        history.redo_stack.push_back(op);
    } else if (keys.just_pressed(KeyCode::KeyY)
        || (keys.just_pressed(KeyCode::KeyZ) && shift))
        && let Some(op) = history.redo_stack.pop_back()
    {
        apply_forward(&op, &mut graph);
        history.undo_stack.push_back(op);
    }
}

fn apply_inverse(op: &Op, graph: &mut TrussGraph) {
    match op {
        Op::AddBeam { beam_id, created_nodes, .. } => {
            graph.beams.remove(beam_id);
            for (id, _) in created_nodes {
                graph.nodes.remove(id);
            }
        }
        Op::DeleteBeam { beam_id, beam, removed_nodes } => {
            for (id, node) in removed_nodes {
                graph.nodes.insert(*id, *node);
            }
            graph.beams.insert(*beam_id, *beam);
        }
        Op::ClearAll { beams, removed_nodes } => {
            for (id, node) in removed_nodes {
                graph.nodes.insert(*id, *node);
            }
            for (id, beam) in beams {
                graph.beams.insert(*id, *beam);
            }
        }
    }
}

fn apply_forward(op: &Op, graph: &mut TrussGraph) {
    match op {
        Op::AddBeam { beam_id, beam, created_nodes } => {
            for (id, node) in created_nodes {
                graph.nodes.insert(*id, *node);
            }
            graph.beams.insert(*beam_id, *beam);
        }
        Op::DeleteBeam { beam_id, removed_nodes, .. } => {
            graph.beams.remove(beam_id);
            for (id, _) in removed_nodes {
                graph.nodes.remove(id);
            }
        }
        Op::ClearAll { beams, removed_nodes } => {
            for (id, _) in beams {
                graph.beams.remove(id);
            }
            for (id, _) in removed_nodes {
                graph.nodes.remove(id);
            }
        }
    }
}

fn clear_all_input(
    keys: Res<ButtonInput<KeyCode>>,
    mut history: ResMut<History>,
    mut graph: ResMut<TrussGraph>,
) {
    let ctrl = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
    if !ctrl || !keys.just_pressed(KeyCode::KeyN) {
        return;
    }
    if graph.beams.is_empty() {
        return;
    }
    let beams: Vec<(BeamId, Beam)> = graph.beams.drain().collect();
    let removed_nodes: Vec<(NodeId, Node)> = graph
        .nodes
        .iter()
        .filter(|(_, n)| n.kind == NodeKind::Free)
        .map(|(id, n)| (*id, *n))
        .collect();
    for (id, _) in &removed_nodes {
        graph.nodes.remove(id);
    }
    history.push(Op::ClearAll { beams, removed_nodes });
}
