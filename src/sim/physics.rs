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
pub const YIELD_STRESS: f32 = 3000.0;

/// Gravitational acceleration applied in the −y direction.
///
/// Scale convention: GRID_SIZE = 32 px ≈ 1 m, so 1 px ≈ 3 cm.
/// Real gravity 9.81 m/s² → 9.81 × 32 ≈ 314 px/s². Rounded to 300.
pub const GRAVITY: f32 = 300.0;
