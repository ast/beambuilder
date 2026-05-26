//! Test-mode physics: a mass-spring Verlet simulation built from a snapshot
//! of the truss graph. Beams act as stiff springs that break past a strain
//! threshold; anchors stay clamped. A small train of coupled point-mass cars
//! interacts with the unbroken beams and decides win/lose.

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

/// Number of train cars (engine + wagons).
pub const CAR_COUNT: usize = 3;
/// Collision radius around each car's center — also the height above the
/// driving surface at which the car's center rides.
pub const CAR_RADIUS: f32 = 20.0;
/// Center-to-center spacing held by couplers. Must be > 2 × body half-width
/// to keep adjacent car bodies from visually overlapping.
pub const CAR_SPACING: f32 = 52.0;
const CAR_MASS: f32 = 1.0;
const DRIVE_FORCE_PER_CAR: f32 = 200.0;
const INITIAL_VX: f32 = 60.0;
/// Number of position-based constraint passes per substep that keep coupled
/// cars at their target spacing.
const COUPLING_ITERATIONS: usize = 4;

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
    pub train: Option<Train>,
    /// Cached terrain polyline (left-to-right) for ground collision.
    pub terrain: Vec<Vec2>,
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

/// A small train of coupled point-mass cars. Index 0 is the engine (front),
/// remaining cars trail behind to the left at fixed spacing.
#[derive(Debug, Clone, Default)]
pub struct Train {
    pub cars: Vec<Car>,
}

#[derive(Debug, Clone, Copy)]
pub struct Car {
    pub pos: Vec2,
    pub prev_pos: Vec2,
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

    let (train, terrain, goal_x) = current
        .as_ref()
        .and_then(|c| levels.get(&c.0))
        .map(|lvl| {
            let head = lvl.vehicle_spawn_vec2();
            let goal = lvl.goal_vec2();
            // Index 0 = engine at vehicle_spawn; remaining cars trail to the left.
            // Bake initial rightward velocity into each prev_pos.
            let v_offset = Vec2::new(INITIAL_VX / 60.0, 0.0);
            let cars: Vec<Car> = (0..CAR_COUNT)
                .map(|i| {
                    let pos = head - Vec2::new(CAR_SPACING * i as f32, 0.0);
                    Car {
                        pos,
                        prev_pos: pos - v_offset,
                        grounded: false,
                    }
                })
                .collect();
            let terrain: Vec<Vec2> = lvl.terrain_vec2().collect();
            (Some(Train { cars }), terrain, goal.x)
        })
        .unwrap_or((None, Vec::new(), f32::INFINITY));

    dyn_state.nodes = nodes;
    dyn_state.beams = beams;
    dyn_state.train = train;
    dyn_state.terrain = terrain;
    dyn_state.goal_x = goal_x;
    dyn_state.result = TestResult::Running;
}

fn exit_test(mut dyn_state: ResMut<DynamicState>) {
    dyn_state.nodes.clear();
    dyn_state.beams.clear();
    dyn_state.train = None;
    dyn_state.terrain.clear();
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

    // ---- Train --------------------------------------------------------------
    let Some(train) = state.train.as_mut() else {
        return;
    };

    // Verlet step each car with gravity + drive force when grounded.
    for car in &mut train.cars {
        let mut force = Vec2::new(0.0, -GRAVITY * CAR_MASS);
        if car.grounded {
            force.x += DRIVE_FORCE_PER_CAR;
        }
        let acc = force / CAR_MASS;
        let velocity = (car.pos - car.prev_pos) * DAMPING;
        let new_pos = car.pos + velocity + acc * dt2;
        car.prev_pos = car.pos;
        car.pos = new_pos;
    }

    // Collide each car against deck-eligible beams + terrain.
    for car in &mut train.cars {
        car.grounded = false;
        // Deck (beams shallow enough to count as a driving surface).
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
            if let Some(push) = resolve_circle_vs_segment(car.pos, CAR_RADIUS, a.pos, b.pos) {
                car.pos += push;
                car.grounded = true;
            }
        }
        // Ground (terrain polyline; pushes upward / outward).
        if let Some(push) = resolve_circle_vs_terrain(car.pos, CAR_RADIUS, &state.terrain) {
            car.pos += push;
            car.grounded = true;
        }
    }

    // Couple consecutive cars with a stiff distance constraint (PBD style).
    for _ in 0..COUPLING_ITERATIONS {
        for i in 0..train.cars.len().saturating_sub(1) {
            let (left, right) = train.cars.split_at_mut(i + 1);
            let a = &mut left[i];
            let b = &mut right[0];
            let delta = b.pos - a.pos;
            let len = delta.length();
            if len < 1e-6 {
                continue;
            }
            let correction = (len - CAR_SPACING) * 0.5 * (delta / len);
            a.pos += correction;
            b.pos -= correction;
        }
    }

    // Win when the trailing car has fully crossed; lose if any car falls.
    let last_x = train.cars.last().map(|c| c.pos.x).unwrap_or(f32::MIN);
    let lowest_y = train.cars.iter().map(|c| c.pos.y).fold(f32::INFINITY, f32::min);
    if last_x >= state.goal_x {
        state.result = TestResult::Won;
    } else if lowest_y < FALL_THRESHOLD {
        state.result = TestResult::Lost;
    }
}

/// Push a car out of any terrain segment it has descended below.
///
/// The terrain polyline runs left-to-right with dirt below it, so the
/// "air-side" normal for each segment is 90° CCW from the segment direction.
/// A segment only acts on cars whose perpendicular projection falls inside
/// its `t ∈ [0, 1]` range — corner handling falls to the neighbouring segment.
/// Among all penetrating segments we return the largest push to keep things
/// simple and well-behaved at concave joints.
fn resolve_circle_vs_terrain(p: Vec2, r: f32, terrain: &[Vec2]) -> Option<Vec2> {
    let mut best: Option<(Vec2, f32)> = None;
    for pair in terrain.windows(2) {
        let a = pair[0];
        let b = pair[1];
        let ab = b - a;
        let len_sq = ab.length_squared();
        if len_sq < 1e-6 {
            continue;
        }
        let len = len_sq.sqrt();
        let t = (p - a).dot(ab) / len_sq;
        if !(0.0..=1.0).contains(&t) {
            continue;
        }
        let n = Vec2::new(-ab.y, ab.x) / len;
        let closest = a + ab * t;
        let signed = (p - closest).dot(n);
        if signed < r {
            let push = n * (r - signed);
            let mag = push.length();
            if best.is_none_or(|(_, m)| mag > m) {
                best = Some((push, mag));
            }
        }
    }
    best.map(|(push, _)| push)
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
