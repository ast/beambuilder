use anyhow::{Context, Result};
use bevy::prelude::Resource;
use clap::Parser;
use std::path::{Path, PathBuf};

/// Beambuilder — a 2D bridge construction game.
///
/// The level argument is a normal filesystem path: absolute if it starts with
/// `/`, otherwise relative to the current working directory — same as `cat`,
/// `ls`, or any other Unix tool. The file must resolve to something under
/// `<project>/assets/`, since that's the asset root Bevy loads from.
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Cli {
    /// Path to a `.level.ron` file. Must resolve under <project>/assets/.
    #[arg(default_value = "assets/levels/01_first_gap.level.ron")]
    pub level: PathBuf,
}

#[derive(thiserror::Error, Debug)]
pub enum CliError {
    #[error("level file is not under the assets directory:\n  file:   {full}\n  assets: {assets}")]
    NotUnderAssets { full: PathBuf, assets: PathBuf },
    #[error(
        "could not locate the assets directory.\n\
         Run via `cargo run` so CARGO_MANIFEST_DIR is set, or place an `assets` \
         directory next to the binary."
    )]
    AssetsDirMissing,
}

/// Path passed through to the level loader. Held as a String because Bevy's
/// `AssetServer::load` takes an asset path string, which must be relative to
/// the asset root.
#[derive(Resource, Debug, Clone)]
pub struct LevelPath(pub String);

impl Cli {
    /// Resolve the requested level to an `(asset_path, full_path)` pair.
    /// `asset_path` is what AssetServer will use (relative to assets/);
    /// `full_path` is the canonical absolute path on disk.
    pub fn resolve(&self) -> Result<(LevelPath, PathBuf)> {
        let full = self
            .level
            .canonicalize()
            .with_context(|| format!("level file not found: {}", self.level.display()))?;

        let assets_root = find_assets_root()?
            .canonicalize()
            .context("canonicalising assets directory")?;

        let rel = full
            .strip_prefix(&assets_root)
            .map_err(|_| CliError::NotUnderAssets {
                full: full.clone(),
                assets: assets_root.clone(),
            })?;

        Ok((LevelPath(rel.to_string_lossy().into_owned()), full))
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
