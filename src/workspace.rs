use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::{Path, PathBuf};

/// Walk up from `start_dir` looking for a Cargo.lock file.
pub fn find_cargo_lock(start_dir: &Path) -> Option<PathBuf> {
    let mut dir = start_dir;
    loop {
        let candidate = dir.join("Cargo.lock");
        if candidate.exists() {
            return Some(candidate);
        }
        dir = dir.parent()?;
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct LockPackage {
    pub name: String,
    pub version: String,
    pub source: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CargoLock {
    #[serde(default)]
    package: Vec<LockPackage>,
}

/// Parse a Cargo.lock file and return only registry packages.
pub fn parse_cargo_lock(path: &Path) -> Result<Vec<LockPackage>> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("reading {}", path.display()))?;
    let lock: CargoLock = toml::from_str(&content)
        .with_context(|| format!("parsing {}", path.display()))?;
    Ok(lock
        .package
        .into_iter()
        .filter(|p| {
            p.source
                .as_deref()
                .is_some_and(|s| s.starts_with("registry+"))
        })
        .collect())
}

/// Extract sorted unique crate names from lock packages.
pub fn workspace_crate_names(packages: &[LockPackage]) -> Vec<String> {
    let mut names: Vec<String> = packages.iter().map(|p| p.name.clone()).collect();
    names.sort_unstable();
    names.dedup();
    names
}
