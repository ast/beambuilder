//! Linear static finite-element solver for the 2D truss graph.
//!
//! Each node has two translational DOFs (x, y). Each beam is a 2-node truss
//! element resisting axial load only — no bending. Anchor DOFs are clamped
//! (Dirichlet BC); all Free node DOFs are degrees of freedom.
//!
//! Solving K_ff · u = f_f (with self-weight gravity loads) yields per-node
//! displacements; per-beam axial stress is E · (Δu · direction) / L.
//!
//! When the constrained system is singular (under-constrained, e.g. a free
//! node with no path to an anchor, or a horizontal chord with no vertical
//! support), Cholesky factorization fails and we report `Unstable`.

use crate::GameState;
use crate::sim::graph::{BeamId, NodeId, NodeKind, TrussGraph};
use crate::sim::physics::{CROSS_SECTION_AREA, DENSITY, GRAVITY, YOUNGS_MODULUS};
use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use nalgebra::{Cholesky, DMatrix, DVector, Matrix4};

/// Computed axial stress per beam (Pa-ish, in our game-tuned units).
#[derive(Resource, Default, Debug)]
pub struct Stresses(pub HashMap<BeamId, f32>);

/// Stability of the constrained truss.
#[derive(Resource, Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum StructureStatus {
    #[default]
    Ok,
    /// K_ff was singular — the truss is under-constrained.
    Unstable,
}

pub struct FemPlugin;

impl Plugin for FemPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Stresses>()
            .init_resource::<StructureStatus>()
            .add_systems(
                Update,
                recompute_static_fem.run_if(in_state(GameState::Edit)),
            );
    }
}

fn recompute_static_fem(
    graph: Res<TrussGraph>,
    mut stresses: ResMut<Stresses>,
    mut status: ResMut<StructureStatus>,
) {
    if !graph.is_changed() {
        return;
    }
    let (s, st) = solve(&graph);
    stresses.0 = s;
    *status = st;
}

/// Per-beam axial direction + length stashed during K assembly so we can
/// compute stresses with the same numbers we used to build the matrix.
struct BeamGeom {
    id: BeamId,
    a_idx: usize,
    b_idx: usize,
    dir_x: f64,
    dir_y: f64,
    length: f64,
}

fn solve(graph: &TrussGraph) -> (HashMap<BeamId, f32>, StructureStatus) {
    if graph.beams.is_empty() {
        return (HashMap::new(), StructureStatus::Ok);
    }

    // Deterministic node ordering so the matrices are reproducible.
    let mut nodes: Vec<NodeId> = graph.nodes.keys().copied().collect();
    nodes.sort_unstable();
    let mut node_index: HashMap<NodeId, usize> = HashMap::new();
    for (i, id) in nodes.iter().enumerate() {
        node_index.insert(*id, i);
    }
    let n = nodes.len();
    let total_dofs = n * 2;

    // Free DOFs: anchor nodes are clamped (x and y).
    let mut free_dofs: Vec<usize> = Vec::new();
    for (i, id) in nodes.iter().enumerate() {
        if graph.nodes[id].kind == NodeKind::Free {
            free_dofs.push(i * 2);
            free_dofs.push(i * 2 + 1);
        }
    }
    if free_dofs.is_empty() {
        return (HashMap::new(), StructureStatus::Ok);
    }

    let e_a = YOUNGS_MODULUS as f64 * CROSS_SECTION_AREA as f64;
    let mut k = DMatrix::<f64>::zeros(total_dofs, total_dofs);
    let mut f = DVector::<f64>::zeros(total_dofs);
    let mut geoms: Vec<BeamGeom> = Vec::with_capacity(graph.beams.len());

    for (bid, beam) in &graph.beams {
        let (Some(pa), Some(pb)) = (graph.node_pos(beam.a), graph.node_pos(beam.b)) else {
            continue;
        };
        let dx = (pb.x - pa.x) as f64;
        let dy = (pb.y - pa.y) as f64;
        let length = (dx * dx + dy * dy).sqrt();
        if length < 1e-6 {
            continue;
        }
        let c = dx / length;
        let s = dy / length;
        let k_axial = e_a / length;

        // 4×4 element stiffness in global coordinates (truss element).
        #[rustfmt::skip]
        let elem = Matrix4::new(
             c * c,  c * s, -c * c, -c * s,
             c * s,  s * s, -c * s, -s * s,
            -c * c, -c * s,  c * c,  c * s,
            -c * s, -s * s,  c * s,  s * s,
        ) * k_axial;

        let a_idx = node_index[&beam.a];
        let b_idx = node_index[&beam.b];
        let dofs = [a_idx * 2, a_idx * 2 + 1, b_idx * 2, b_idx * 2 + 1];
        for (r, &gr) in dofs.iter().enumerate() {
            for (col, &gc) in dofs.iter().enumerate() {
                k[(gr, gc)] += elem[(r, col)];
            }
        }

        // Self-weight: half the beam's mass loads each endpoint vertically (−y).
        let mass = DENSITY as f64 * CROSS_SECTION_AREA as f64 * length;
        let f_node = -(GRAVITY as f64) * mass * 0.5;
        f[a_idx * 2 + 1] += f_node;
        f[b_idx * 2 + 1] += f_node;

        geoms.push(BeamGeom {
            id: *bid,
            a_idx,
            b_idx,
            dir_x: c,
            dir_y: s,
            length,
        });
    }

    // Build K_ff (free-free) and f_f.
    let m = free_dofs.len();
    let mut k_ff = DMatrix::<f64>::zeros(m, m);
    let mut f_f = DVector::<f64>::zeros(m);
    for (i, &gi) in free_dofs.iter().enumerate() {
        f_f[i] = f[gi];
        for (j, &gj) in free_dofs.iter().enumerate() {
            k_ff[(i, j)] = k[(gi, gj)];
        }
    }

    // Solve. Cholesky returns None for non-positive-definite matrices, which
    // for a symmetric truss stiffness matrix means under-constrained.
    let Some(chol) = Cholesky::new(k_ff) else {
        return (HashMap::new(), StructureStatus::Unstable);
    };
    let u_f = chol.solve(&f_f);

    // Reconstruct full displacement vector (anchor DOFs stay 0).
    let mut u = DVector::<f64>::zeros(total_dofs);
    for (i, &gi) in free_dofs.iter().enumerate() {
        u[gi] = u_f[i];
    }

    let mut stresses = HashMap::new();
    for g in &geoms {
        let du_x = u[g.b_idx * 2] - u[g.a_idx * 2];
        let du_y = u[g.b_idx * 2 + 1] - u[g.a_idx * 2 + 1];
        let axial = du_x * g.dir_x + du_y * g.dir_y;
        let strain = axial / g.length;
        let stress = (YOUNGS_MODULUS as f64 * strain) as f32;
        stresses.insert(g.id, stress);
    }

    (stresses, StructureStatus::Ok)
}
