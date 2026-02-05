use anyhow::{Context, Result};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::Path;

use crate::prefix;

#[derive(Debug, Clone, Deserialize)]
pub struct IndexDep {
    pub name: String,
    pub req: String,
    #[serde(default)]
    pub features: Vec<String>,
    #[serde(default)]
    pub optional: bool,
    #[serde(default)]
    pub default_features: bool,
    pub target: Option<String>,
    #[serde(default = "default_kind")]
    pub kind: String,
    pub package: Option<String>,
}

fn default_kind() -> String {
    "normal".to_string()
}

#[derive(Debug, Clone, Deserialize)]
pub struct IndexEntry {
    pub name: String,
    pub vers: String,
    #[serde(default)]
    pub deps: Vec<IndexDep>,
    #[serde(default)]
    pub cksum: String,
    #[serde(default)]
    pub features: BTreeMap<String, Vec<String>>,
    #[serde(default)]
    pub features2: BTreeMap<String, Vec<String>>,
    #[serde(default)]
    pub yanked: bool,
    #[serde(default)]
    pub pubtime: Option<String>,
}

impl IndexEntry {
    /// Merged features map (features + features2).
    pub fn all_features(&self) -> BTreeMap<String, Vec<String>> {
        let mut merged = self.features.clone();
        for (k, v) in &self.features2 {
            merged.entry(k.clone()).or_default().extend(v.iter().cloned());
        }
        merged
    }
}

/// Read all versions from a crate's index file.
pub fn read_crate_index(index_root: &Path, name: &str) -> Result<Vec<IndexEntry>> {
    let path = prefix::index_path(index_root, name);
    let content = std::fs::read_to_string(&path)
        .with_context(|| format!("reading index for '{}' at {}", name, path.display()))?;

    let mut entries = Vec::new();
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let entry: IndexEntry = serde_json::from_str(line)
            .with_context(|| format!("parsing index line for '{}'", name))?;
        entries.push(entry);
    }
    Ok(entries)
}

/// Read the latest non-yanked version of a crate.
pub fn read_latest_version(index_root: &Path, name: &str) -> Result<IndexEntry> {
    let entries = read_crate_index(index_root, name)?;
    entries
        .into_iter()
        .rev()
        .find(|e| !e.yanked)
        .ok_or_else(|| anyhow::anyhow!("no non-yanked versions found for '{}'", name))
}
