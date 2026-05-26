use crate::world::level::{CurrentLevel, Level};
use bevy::platform::collections::HashMap;
use bevy::prelude::*;

pub type NodeId = u32;
pub type BeamId = u32;

/// Maximum permissible beam length in world units (6 grid cells of 32).
/// Forces players to span gaps with multiple beams + intermediate nodes.
pub const MAX_BEAM_LENGTH: f32 = 192.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NodeKind {
    Anchor,
    Free,
}

#[derive(Clone, Copy, Debug)]
pub struct Node {
    pub pos: Vec2,
    pub kind: NodeKind,
}

#[derive(Clone, Copy, Debug)]
pub struct Beam {
    pub a: NodeId,
    pub b: NodeId,
}

/// Canonical truss model. Single source of truth for FEM (edit mode) and the
/// dynamic mass-spring sim (test mode) — both read this; neither mutates it.
#[derive(Resource, Default, Debug)]
pub struct TrussGraph {
    pub nodes: HashMap<NodeId, Node>,
    pub beams: HashMap<BeamId, Beam>,
    next_node: u32,
    next_beam: u32,
}

impl TrussGraph {
    pub fn add_node(&mut self, pos: Vec2, kind: NodeKind) -> NodeId {
        let id = self.next_node;
        self.next_node += 1;
        self.nodes.insert(id, Node { pos, kind });
        id
    }

    pub fn add_beam(&mut self, a: NodeId, b: NodeId) -> Option<BeamId> {
        if a == b {
            return None;
        }
        if self.beams.values().any(|b2| same_endpoints(*b2, a, b)) {
            return None;
        }
        let id = self.next_beam;
        self.next_beam += 1;
        self.beams.insert(id, Beam { a, b });
        Some(id)
    }

    pub fn remove_beam(&mut self, id: BeamId) -> Option<Beam> {
        self.beams.remove(&id)
    }

    pub fn remove_node(&mut self, id: NodeId) -> Option<Node> {
        self.nodes.remove(&id)
    }

    pub fn node_pos(&self, id: NodeId) -> Option<Vec2> {
        self.nodes.get(&id).map(|n| n.pos)
    }

    /// Count beams attached to a node — used to clean up orphan Free nodes.
    pub fn beam_count_for(&self, id: NodeId) -> usize {
        self.beams
            .values()
            .filter(|b| b.a == id || b.b == id)
            .count()
    }
}

fn same_endpoints(beam: Beam, a: NodeId, b: NodeId) -> bool {
    (beam.a == a && beam.b == b) || (beam.a == b && beam.b == a)
}

pub struct GraphPlugin;

impl Plugin for GraphPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TrussGraph>()
            .insert_resource(AnchorsSeeded(false))
            .add_systems(Update, seed_anchors_from_level);
    }
}

/// Flips to `true` after we've seeded anchors from the loaded level so the
/// seeding system stops running.
#[derive(Resource)]
struct AnchorsSeeded(bool);

fn seed_anchors_from_level(
    current: Option<Res<CurrentLevel>>,
    levels: Res<Assets<Level>>,
    mut graph: ResMut<TrussGraph>,
    mut seeded: ResMut<AnchorsSeeded>,
) {
    if seeded.0 {
        return;
    }
    let Some(handle) = current.as_ref().map(|c| &c.0) else {
        return;
    };
    let Some(level) = levels.get(handle) else {
        return;
    };
    for pos in level.anchors_vec2() {
        graph.add_node(pos, NodeKind::Anchor);
    }
    seeded.0 = true;
}
