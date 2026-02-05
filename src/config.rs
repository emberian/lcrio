use std::path::PathBuf;

#[derive(Debug, Clone)]
pub enum CrateSource {
    Panamax { root: PathBuf },
    Cargo { cargo_home: PathBuf, registry_hash: String },
}

#[derive(Debug, Clone)]
pub struct LcrioConfig {
    pub source: CrateSource,
    pub cache_dir: PathBuf,
    pub search_limit: usize,
    pub workspace_filter: Option<Vec<String>>,
}

impl Default for LcrioConfig {
    fn default() -> Self {
        Self::auto_detect()
    }
}

impl LcrioConfig {
    /// Auto-detect crate source: panamax if ~/crates.io/full exists, else cargo registry.
    pub fn auto_detect() -> Self {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
        let cache_dir = PathBuf::from(&home).join(".cache/lcrio");

        let panamax_root = PathBuf::from(&home).join("crates.io/full");
        if panamax_root.join("crates.io-index").exists() {
            return Self {
                source: CrateSource::Panamax { root: panamax_root },
                cache_dir,
                search_limit: 20,
                workspace_filter: None,
            };
        }

        let cargo_home = PathBuf::from(&home).join(".cargo");
        if let Some(hash) = discover_registry_hash(&cargo_home) {
            return Self {
                source: CrateSource::Cargo { cargo_home, registry_hash: hash },
                cache_dir,
                search_limit: 20,
                workspace_filter: None,
            };
        }

        // Fall back to panamax even if it doesn't exist (will error on use)
        Self {
            source: CrateSource::Panamax { root: panamax_root },
            cache_dir,
            search_limit: 20,
            workspace_filter: None,
        }
    }

    pub fn with_source(source: CrateSource) -> Self {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
        Self {
            source,
            cache_dir: PathBuf::from(&home).join(".cache/lcrio"),
            search_limit: 20,
            workspace_filter: None,
        }
    }

    pub fn index_root(&self) -> PathBuf {
        match &self.source {
            CrateSource::Panamax { root } => root.join("crates.io-index"),
            CrateSource::Cargo { cargo_home, registry_hash } => {
                cargo_home.join("registry/index").join(registry_hash).join(".cache")
            }
        }
    }

    pub fn crates_root(&self) -> PathBuf {
        match &self.source {
            CrateSource::Panamax { root } => root.join("crates"),
            CrateSource::Cargo { cargo_home, registry_hash } => {
                cargo_home.join("registry/cache").join(registry_hash)
            }
        }
    }

    /// Source root for pre-extracted crate sources (cargo mode only).
    pub fn source_root(&self) -> Option<PathBuf> {
        match &self.source {
            CrateSource::Panamax { .. } => None,
            CrateSource::Cargo { cargo_home, registry_hash } => {
                Some(cargo_home.join("registry/src").join(registry_hash))
            }
        }
    }

    pub fn is_cargo(&self) -> bool {
        matches!(&self.source, CrateSource::Cargo { .. })
    }

    pub fn source_label(&self) -> &'static str {
        match &self.source {
            CrateSource::Panamax { .. } => "panamax",
            CrateSource::Cargo { .. } => "cargo",
        }
    }
}

/// Discover the registry hash directory under ~/.cargo/registry/src/.
/// Looks for the first directory matching `index.crates.io-*`.
fn discover_registry_hash(cargo_home: &std::path::Path) -> Option<String> {
    let src_dir = cargo_home.join("registry/src");
    let entries = std::fs::read_dir(&src_dir).ok()?;
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with("index.crates.io-") && entry.file_type().ok()?.is_dir() {
            return Some(name);
        }
    }
    None
}
