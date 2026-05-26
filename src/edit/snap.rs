use crate::edit::grid::snap_to_grid;
use crate::sim::graph::{NodeId, TrussGraph};
use bevy::prelude::*;

/// World-unit distance at which the cursor snaps to an existing node.
pub const SNAP_RADIUS: f32 = 16.0;

#[derive(Clone, Copy, Debug)]
pub enum SnapResult {
    /// Cursor is on an existing node; second field is that node's position.
    Existing(NodeId, Vec2),
    /// Cursor snapped to a grid cell at this position.
    Grid(Vec2),
}

impl SnapResult {
    pub fn pos(self) -> Vec2 {
        match self {
            SnapResult::Existing(_, p) | SnapResult::Grid(p) => p,
        }
    }
}

/// Snap the cursor to the nearest existing node (within [`SNAP_RADIUS`]),
/// falling back to grid snapping.
pub fn snap(world: Vec2, graph: &TrussGraph) -> SnapResult {
    let mut best: Option<(NodeId, Vec2, f32)> = None;
    for (id, n) in &graph.nodes {
        let d = (n.pos - world).length();
        if d < SNAP_RADIUS && best.is_none_or(|(_, _, bd)| d < bd) {
            best = Some((*id, n.pos, d));
        }
    }
    if let Some((id, p, _)) = best {
        return SnapResult::Existing(id, p);
    }
    SnapResult::Grid(snap_to_grid(world))
}

/// Translate the cursor's window position into world coordinates.
pub fn world_cursor(
    windows: &Query<&Window>,
    cameras: &Query<(&Camera, &GlobalTransform)>,
) -> Option<Vec2> {
    let window = windows.single().ok()?;
    let cursor = window.cursor_position()?;
    let (camera, transform) = cameras.single().ok()?;
    camera.viewport_to_world_2d(transform, cursor).ok()
}
