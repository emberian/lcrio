pub mod browse;
pub mod config;
pub mod index;
pub mod prefix;
pub mod search;
pub mod unpack;

use config::LcrioConfig;
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

    /// Get or build the sorted name index.
    pub fn names(&self) -> &[String] {
        self.name_index.get_or_init(|| {
            search::build_name_index(&self.config.index_root())
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
        index::read_crate_index(&self.config.index_root(), name)
    }

    /// Read the latest non-yanked version.
    pub fn latest_version(&self, name: &str) -> anyhow::Result<index::IndexEntry> {
        index::read_latest_version(&self.config.index_root(), name)
    }

    /// Unpack a specific version of a crate.
    pub fn unpack(&self, name: &str, version: &str) -> anyhow::Result<unpack::UnpackResult> {
        unpack::unpack_crate(
            &self.config.crates_root(),
            &self.config.cache_dir,
            name,
            version,
        )
    }

    /// Unpack the latest version of a crate.
    pub fn unpack_latest(&self, name: &str) -> anyhow::Result<(unpack::UnpackResult, String)> {
        unpack::unpack_latest(
            &self.config.index_root(),
            &self.config.crates_root(),
            &self.config.cache_dir,
            name,
        )
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
