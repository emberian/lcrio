pub mod browse;
pub mod config;
pub mod index;
pub mod prefix;
pub mod search;
pub mod unpack;
pub mod workspace;

use config::{CrateSource, LcrioConfig};
use std::sync::OnceLock;

pub struct Lcrio {
    pub config: LcrioConfig,
    name_index: OnceLock<Vec<String>>,
}

impl Lcrio {
    pub fn new(config: LcrioConfig) -> Self {
        Self {
            config,
            name_index: OnceLock::new(),
        }
    }

    pub fn with_defaults() -> Self {
        Self::new(LcrioConfig::default())
    }

    /// Construct with explicit source and workspace options.
    ///
    /// - `source`: if None, auto-detect; if Some, use that source.
    /// - `workspace`: if true, find Cargo.lock and filter to workspace deps.
    pub fn with_options(source: Option<CrateSource>, workspace: bool) -> Self {
        let mut config = match source {
            Some(s) => LcrioConfig::with_source(s),
            None => LcrioConfig::auto_detect(),
        };

        if workspace {
            if let Ok(cwd) = std::env::current_dir() {
                if let Some(lock_path) = workspace::find_cargo_lock(&cwd) {
                    if let Ok(packages) = workspace::parse_cargo_lock(&lock_path) {
                        let names = workspace::workspace_crate_names(&packages);
                        if !names.is_empty() {
                            config.workspace_filter = Some(names);
                        }
                    }
                }
            }
        }

        Self::new(config)
    }

    /// Get or build the sorted name index, optionally filtered to workspace deps.
    pub fn names(&self) -> &[String] {
        self.name_index.get_or_init(|| {
            let mut names = search::build_name_index(&self.config.index_root());

            // In cargo mode, also merge names from src/ directory
            if let Some(src_root) = self.config.source_root() {
                let src_names = search::build_name_index_from_src(&src_root);
                for name in src_names {
                    if names.binary_search(&name).is_err() {
                        names.push(name);
                    }
                }
                names.sort_unstable();
                names.dedup();
            }

            // Apply workspace filter if set
            if let Some(ref filter) = self.config.workspace_filter {
                names.retain(|n| filter.binary_search(n).is_ok());
            }

            names
        })
    }

    /// Search for crate names.
    pub fn search(&self, query: &str, limit: Option<usize>) -> Vec<search::SearchResult> {
        let limit = limit.unwrap_or(self.config.search_limit);
        search::search_names(self.names(), query, limit)
    }

    /// Search for crates that depend on a given crate.
    pub fn search_by_dep(&self, dep_name: &str, limit: Option<usize>) -> Vec<String> {
        let limit = limit.unwrap_or(self.config.search_limit);
        search::search_by_dep(&self.config.index_root(), self.names(), dep_name, limit)
    }

    /// Search for crates with a given feature.
    pub fn search_by_feature(&self, feature_name: &str, limit: Option<usize>) -> Vec<String> {
        let limit = limit.unwrap_or(self.config.search_limit);
        search::search_by_feature(&self.config.index_root(), self.names(), feature_name, limit)
    }

    /// Read all index entries for a crate.
    pub fn crate_index(&self, name: &str) -> anyhow::Result<Vec<index::IndexEntry>> {
        if self.config.is_cargo() {
            index::read_cargo_sparse_index(&self.config.index_root(), name)
        } else {
            index::read_crate_index(&self.config.index_root(), name)
        }
    }

    /// Read the latest non-yanked version.
    pub fn latest_version(&self, name: &str) -> anyhow::Result<index::IndexEntry> {
        if self.config.is_cargo() {
            index::read_latest_version_sparse(&self.config.index_root(), name)
        } else {
            index::read_latest_version(&self.config.index_root(), name)
        }
    }

    /// Unpack a specific version of a crate.
    pub fn unpack(&self, name: &str, version: &str) -> anyhow::Result<unpack::UnpackResult> {
        // In cargo mode, try pre-extracted source first
        if let Some(src_root) = self.config.source_root() {
            if let Ok(result) = unpack::cargo_source_path(&src_root, name, version) {
                return Ok(result);
            }
        }

        // Fall back to .crate extraction
        let crates_root = self.config.crates_root();
        unpack::unpack_crate(
            &crates_root,
            &self.config.cache_dir,
            name,
            version,
        )
    }

    /// Unpack the latest version of a crate.
    pub fn unpack_latest(&self, name: &str) -> anyhow::Result<(unpack::UnpackResult, String)> {
        let latest = self.latest_version(name)?;
        let result = self.unpack(name, &latest.vers)?;
        Ok((result, latest.vers))
    }

    /// List files in an unpacked crate.
    pub fn list_files(
        &self,
        name: &str,
        version: &str,
        pattern: Option<&str>,
    ) -> anyhow::Result<Vec<browse::FileEntry>> {
        let result = self.unpack(name, version)?;
        browse::list_files(&result.path, pattern)
    }

    /// Read a file from an unpacked crate.
    pub fn read_file(
        &self,
        name: &str,
        version: &str,
        path: &str,
    ) -> anyhow::Result<String> {
        let result = self.unpack(name, version)?;
        browse::read_file(&result.path, path)
    }

    /// Clean the extraction cache.
    pub fn clean(&self) -> anyhow::Result<()> {
        unpack::clean_cache(&self.config.cache_dir)
    }
}
