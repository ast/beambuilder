//! Game-tuned physical constants. Not SI — picked so that stress colors
//! light up meaningfully at our world-unit/pixel scale (GRID_SIZE = 32).

/// Young's modulus (axial stiffness coefficient).
pub const YOUNGS_MODULUS: f32 = 2.0e5;

/// Beam cross-section area. Used for stiffness (k = EA/L) and self-weight (m = ρAL).
pub const CROSS_SECTION_AREA: f32 = 1.0;

/// Mass density of beam material.
pub const DENSITY: f32 = 0.005;

/// Stress at which a beam is "fully red" (yield). Used only for visualization in M4;
/// dynamic breakage in M5 may use a separate ultimate-strength constant.
pub const YIELD_STRESS: f32 = 100.0;

/// Gravitational acceleration applied in the −y direction.
pub const GRAVITY: f32 = 9.81;
