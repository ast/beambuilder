# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

Beambuilder is a 2D bridge construction game in Rust, inspired by the original Pontifex / Bridge Builder (wireframe trusses on dark background, color-coded stress, watch your bridge break under a passing vehicle). Target audience is older kids — a "simple CAD with challenges and simulations."

The classic 2D aesthetic is the goal. Do not add 3D, parallax scrolling, particle effects, or other modern visual noise.

## Tech stack (locked in)

- **Engine:** Bevy 0.18 (ECS, 2D).
- **Physics:** Hybrid. Static linear FEM (truss stiffness matrix solved with `nalgebra`) runs live during edit mode for stress coloring. Dynamic mass-spring with Verlet integration runs during test mode so beams visibly sag and snap. Do **not** reach for `rapier` — its joints don't model axial truss stress well and the user explicitly wants a CAD-like FEM feel.
- **Levels:** RON files in `assets/levels/`, loaded via `bevy_common_assets`. Adding a level must never require recompiling.
- **Single beam type** in MVP. Multi-material (cable, road deck, steel) is explicitly deferred.
- **Platform:** Desktop only (Linux/Mac/Windows). No wasm/mobile for MVP.
- **Loads in MVP:** Gravity + one rolling vehicle. Win condition = vehicle reaches the goal anchor without the bridge collapsing.

## Commands

```bash
cargo run                                    # play the game
cargo build --release                        # optimized build
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt
```

Bevy is slow to compile in debug. Once dependencies land, add this to `Cargo.toml`:

```toml
[profile.dev]
opt-level = 1
[profile.dev.package."*"]
opt-level = 3
```

## Architecture

Most files do not exist yet — this is the planned layout, create modules as the work reaches them:

- `src/main.rs` — App setup, plugin wiring, `GameState` enum (`Menu` / `Edit` / `Test` / `Result`).
- `src/world/` — `Level` RON asset + loader, anchor points, terrain polyline.
- `src/edit/` — Grid snapping, beam add/delete, undo/redo, mouse input.
- `src/sim/` — Truss graph (`Node`, `Beam`), `fem.rs` static solver, `dynamic.rs` Verlet sim with breakage, `vehicle.rs` rolling load.
- `src/render/` — Beam drawing, stress coloring (green → yellow → red), anchor/terrain rendering.
- `src/ui/` — HUD (Edit/Test/Pause chips), level select screen.
- `assets/levels/*.ron` — Hand-authored levels.

The **truss graph is the single source of truth** shared between FEM and dynamic sims. Both evaluators read the same `Node`/`Beam` data — they are different solvers over one model, not parallel data structures.

## Conventions

- Use Bevy idioms: components are plain structs, logic lives in systems, cross-cutting concerns become plugins.
- Edit-mode and test-mode systems must be gated by `GameState` run conditions, not by internal flags.
- Levels are data, not code.
