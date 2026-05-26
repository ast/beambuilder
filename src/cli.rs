use anyhow::{Context, Result};
use bevy::prelude::Resource;
use clap::Parser;
use std::path::{Path, PathBuf};

/// Beambuilder — a 2D bridge construction game.
///
/// Pass an alternative level via the positional argument. The path is treated
/// as relative to the project's `assets/` directory unless it is absolute,
/// in which case it must already live under `assets/` (or a symlink there).
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Cli {
    /// Level file (relative to assets/, e.g. levels/02_wide_gap.level.ron).
    #[arg(default_value = "levels/01_first_gap.level.ron")]
    pub level: PathBuf,
}

#[derive(thiserror::Error, Debug)]
pub enum CliError {
    #[error(
        "level file not found: {asset_path}\n  expected at: {full_path}\n\
         hint: levels live under <project>/assets/, run with `cargo run -- <path>`"
    )]
    LevelNotFound {
        asset_path: String,
        full_path: PathBuf,
    },
    #[error(
        "could not locate the assets directory.\n\
         Run via `cargo run` so CARGO_MANIFEST_DIR is set, or place an `assets` \
         directory next to the binary."
    )]
    AssetsDirMissing,
}

/// Path passed through to the level loader. Held as a String because Bevy's
/// `AssetServer::load` takes an asset path string.
#[derive(Resource, Debug, Clone)]
pub struct LevelPath(pub String);

impl Cli {
    /// Resolve the requested level to an `(asset_path, full_path)` pair.
    /// `asset_path` is the path AssetServer will use (relative to assets/);
    /// `full_path` is the absolute path on disk used for the existence check.
    pub fn resolve(&self) -> Result<(LevelPath, PathBuf)> {
        let assets_root = find_assets_root()?;
        let asset_path = self.level.to_string_lossy().to_string();
        let full = assets_root.join(&asset_path);
        if !full.exists() {
            return Err(CliError::LevelNotFound {
                asset_path,
                full_path: full,
            })
            .context("validating CLI arguments");
        }
        Ok((LevelPath(asset_path), full))
    }
}

/// Find the `assets/` directory. Tries `CARGO_MANIFEST_DIR/assets` first
/// (the normal `cargo run` path), then `./assets`, then `<exe>/../assets`.
fn find_assets_root() -> Result<PathBuf> {
    if let Ok(manifest) = std::env::var("CARGO_MANIFEST_DIR") {
        let p = Path::new(&manifest).join("assets");
        if p.is_dir() {
            return Ok(p);
        }
    }
    let cwd = Path::new("assets");
    if cwd.is_dir() {
        return Ok(cwd.canonicalize().unwrap_or_else(|_| cwd.to_path_buf()));
    }
    if let Ok(exe) = std::env::current_exe()
        && let Some(parent) = exe.parent()
    {
        let p = parent.join("assets");
        if p.is_dir() {
            return Ok(p);
        }
    }
    Err(CliError::AssetsDirMissing).context("locating assets directory")
}
