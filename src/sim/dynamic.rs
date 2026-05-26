//! Test-mode physics: a mass-spring Verlet simulation built from a snapshot
//! of the truss graph. Beams act as stiff springs that break past a strain
//! threshold; anchors stay clamped. A point-mass vehicle interacts with the
//! unbroken beams and decides win/lose.

use crate::GameState;
use crate::sim::graph::{BeamId, NodeId, NodeKind, TrussGraph};
use crate::sim::physics::{CROSS_SECTION_AREA, DENSITY, GRAVITY, YOUNGS_MODULUS};
use crate::world::level::{CurrentLevel, Level};
use bevy::platform::collections::HashMap;
use bevy::prelude::*;

const SUBSTEPS: usize = 8;
const DAMPING: f32 = 0.9995;
const BREAK_STRAIN: f32 = 0.05;
const FALL_THRESHOLD: f32 = -500.0;

const VEHICLE_RADIUS: f32 = 10.0;
const VEHICLE_MASS: f32 = 2.0;
const VEHICLE_DRIVE_FORCE: f32 = 400.0;
const VEHICLE_INITIAL_VX: f32 = 60.0;

/// Beams steeper than this (sine of slope angle) don't collide with the vehicle.
/// Models the original Bridge Builder convention: think of the bridge as a 3D side
/// view — the deck is the horizontal driving surface, struts/diagonals/top chord
/// are out-of-plane structural members and the vehicle drives between/past them.
/// 0.5 ≈ 30°, so 45° diagonals and 90° verticals are skipped; only roughly
/// horizontal beams act as a driving surface.
const MAX_DECK_SLOPE: f32 = 0.5;

/// Mass used in place of zero for non-anchor nodes that wound up massless.
const MIN_FREE_MASS: f32 = 0.001;

#[derive(Resource, Default, Debug)]
pub struct DynamicState {
    pub nodes: HashMap<NodeId, DynNode>,
    pub beams: HashMap<BeamId, DynBeam>,
    pub vehicle: Option<Vehicle>,
    pub goal_x: f32,
    pub result: TestResult,
}

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum TestResult {
    #[default]
    Running,
    Won,
    Lost,
}

#[derive(Debug, Clone, Copy)]
pub struct DynNode {
    pub pos: Vec2,
    pub prev_pos: Vec2,
    pub mass: f32,
    pub fixed: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct DynBeam {
    pub a: NodeId,
    pub b: NodeId,
    pub rest_length: f32,
    pub broken: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct Vehicle {
    pub pos: Vec2,
    pub prev_pos: Vec2,
    pub radius: f32,
    pub mass: f32,
    pub grounded: bool,
}

pub struct DynamicPlugin;

impl Plugin for DynamicPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DynamicState>()
            .add_systems(Update, toggle_test_state)
            .add_systems(OnEnter(GameState::Test), enter_test)
            .add_systems(OnExit(GameState::Test), exit_test)
            .add_systems(
                Update,
                step_dynamics.run_if(in_state(GameState::Test)),
            );
    }
}

fn toggle_test_state(
    keys: Res<ButtonInput<KeyCode>>,
    state: Res<State<GameState>>,
    mut next: ResMut<NextState<GameState>>,
) {
    match state.get() {
        GameState::Edit if keys.just_pressed(KeyCode::Space) => {
            next.set(GameState::Test);
        }
        GameState::Test if keys.just_pressed(KeyCode::Escape) => {
            next.set(GameState::Edit);
        }
        _ => {}
    }
}

fn enter_test(
    graph: Res<TrussGraph>,
    current: Option<Res<CurrentLevel>>,
    levels: Res<Assets<Level>>,
    mut dyn_state: ResMut<DynamicState>,
) {
    let mut nodes: HashMap<NodeId, DynNode> = HashMap::new();
    let mut beams: HashMap<BeamId, DynBeam> = HashMap::new();

    for (id, node) in &graph.nodes {
        nodes.insert(
            *id,
            DynNode {
                pos: node.pos,
                prev_pos: node.pos,
                mass: 0.0,
                fixed: node.kind == NodeKind::Anchor,
            },
        );
    }

    for (bid, beam) in &graph.beams {
        let (Some(pa), Some(pb)) = (graph.node_pos(beam.a), graph.node_pos(beam.b)) else {
            continue;
        };
        let rest_length = (pb - pa).length();
        let half_mass = DENSITY * CROSS_SECTION_AREA * rest_length * 0.5;
        if let Some(n) = nodes.get_mut(&beam.a) {
            n.mass += half_mass;
        }
        if let Some(n) = nodes.get_mut(&beam.b) {
            n.mass += half_mass;
        }
        beams.insert(
            *bid,
            DynBeam {
                a: beam.a,
                b: beam.b,
                rest_length,
                broken: false,
            },
        );
    }

    for node in nodes.values_mut() {
        if node.fixed {
            // Anchors are never integrated, so the mass value is irrelevant —
            // but keep it nonzero so accidental access doesn't divide by 0.
            node.mass = 1.0;
        } else if node.mass < MIN_FREE_MASS {
            node.mass = MIN_FREE_MASS;
        }
    }

    let (vehicle, goal_x) = current
        .as_ref()
        .and_then(|c| levels.get(&c.0))
        .map(|lvl| {
            let pos = lvl.vehicle_spawn_vec2();
            let goal = lvl.goal_vec2();
            // Bake an initial rightward velocity into prev_pos so Verlet picks it up.
            let prev_pos = pos - Vec2::new(VEHICLE_INITIAL_VX / 60.0, 0.0);
            (
                Some(Vehicle {
                    pos,
                    prev_pos,
                    radius: VEHICLE_RADIUS,
                    mass: VEHICLE_MASS,
                    grounded: false,
                }),
                goal.x,
            )
        })
        .unwrap_or((None, f32::INFINITY));

    dyn_state.nodes = nodes;
    dyn_state.beams = beams;
    dyn_state.vehicle = vehicle;
    dyn_state.goal_x = goal_x;
    dyn_state.result = TestResult::Running;
}

fn exit_test(mut dyn_state: ResMut<DynamicState>) {
    dyn_state.nodes.clear();
    dyn_state.beams.clear();
    dyn_state.vehicle = None;
    dyn_state.goal_x = 0.0;
    dyn_state.result = TestResult::Running;
}

fn step_dynamics(time: Res<Time>, mut dyn_state: ResMut<DynamicState>) {
    if dyn_state.result != TestResult::Running {
        return;
    }
    let frame_dt = time.delta_secs().min(1.0 / 30.0); // hard cap to keep stability across hitches
    let dt = frame_dt / SUBSTEPS as f32;
    for _ in 0..SUBSTEPS {
        substep(&mut dyn_state, dt);
        if dyn_state.result != TestResult::Running {
            break;
        }
    }
}

fn substep(state: &mut DynamicState, dt: f32) {
    let dt2 = dt * dt;

    // ---- Forces on nodes ----------------------------------------------------
    let mut forces: HashMap<NodeId, Vec2> = state
        .nodes
        .iter()
        .map(|(id, n)| {
            let f = if n.fixed {
                Vec2::ZERO
            } else {
                Vec2::new(0.0, -GRAVITY * n.mass)
            };
            (*id, f)
        })
        .collect();

    let mut newly_broken: Vec<BeamId> = Vec::new();
    for (bid, beam) in &state.beams {
        if beam.broken {
            continue;
        }
        let (Some(a), Some(b)) = (state.nodes.get(&beam.a), state.nodes.get(&beam.b)) else {
            continue;
        };
        let delta = b.pos - a.pos;
        let len = delta.length();
        if len < 1e-6 {
            continue;
        }
        let dir = delta / len;
        let stretch = len - beam.rest_length;
        let strain = stretch / beam.rest_length;
        if strain.abs() > BREAK_STRAIN {
            newly_broken.push(*bid);
            continue;
        }
        let k = YOUNGS_MODULUS * CROSS_SECTION_AREA / beam.rest_length;
        let f = dir * (k * stretch);
        if let Some(fa) = forces.get_mut(&beam.a) {
            *fa += f;
        }
        if let Some(fb) = forces.get_mut(&beam.b) {
            *fb -= f;
        }
    }
    for bid in newly_broken {
        if let Some(beam) = state.beams.get_mut(&bid) {
            beam.broken = true;
        }
    }

    // ---- Verlet integrate nodes --------------------------------------------
    for (id, node) in state.nodes.iter_mut() {
        if node.fixed {
            continue;
        }
        let force = forces.get(id).copied().unwrap_or_default();
        let acc = force / node.mass;
        let velocity = (node.pos - node.prev_pos) * DAMPING;
        let new_pos = node.pos + velocity + acc * dt2;
        node.prev_pos = node.pos;
        node.pos = new_pos;
    }

    // ---- Vehicle ------------------------------------------------------------
    if let Some(vehicle) = state.vehicle.as_mut() {
        let mut force = Vec2::new(0.0, -GRAVITY * vehicle.mass);
        if vehicle.grounded {
            force.x += VEHICLE_DRIVE_FORCE;
        }
        let acc = force / vehicle.mass;
        let velocity = (vehicle.pos - vehicle.prev_pos) * DAMPING;
        let new_pos = vehicle.pos + velocity + acc * dt2;
        vehicle.prev_pos = vehicle.pos;
        vehicle.pos = new_pos;

        // Collide against every unbroken beam that's shallow enough to be a deck.
        vehicle.grounded = false;
        for beam in state.beams.values() {
            if beam.broken {
                continue;
            }
            let (Some(a), Some(b)) = (state.nodes.get(&beam.a), state.nodes.get(&beam.b)) else {
                continue;
            };
            let delta = b.pos - a.pos;
            let len = delta.length();
            if len < 1e-6 || (delta.y / len).abs() > MAX_DECK_SLOPE {
                continue;
            }
            if let Some(push) = resolve_circle_vs_segment(vehicle.pos, vehicle.radius, a.pos, b.pos)
            {
                vehicle.pos += push;
                vehicle.grounded = true;
            }
        }

        // Win / lose checks
        if vehicle.pos.x >= state.goal_x {
            state.result = TestResult::Won;
        } else if vehicle.pos.y < FALL_THRESHOLD {
            state.result = TestResult::Lost;
        }
    }
}

/// If the circle penetrates the segment, return the minimum push-out vector.
fn resolve_circle_vs_segment(p: Vec2, r: f32, a: Vec2, b: Vec2) -> Option<Vec2> {
    let ab = b - a;
    let len_sq = ab.length_squared();
    if len_sq < 1e-6 {
        return None;
    }
    let t = ((p - a).dot(ab) / len_sq).clamp(0.0, 1.0);
    let closest = a + ab * t;
    let to_p = p - closest;
    let d = to_p.length();
    if d >= r {
        return None;
    }
    let normal = if d > 1e-6 { to_p / d } else { Vec2::Y };
    Some(normal * (r - d))
}
