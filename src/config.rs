use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct LcrioConfig {
    pub panamax_root: PathBuf,
    pub cache_dir: PathBuf,
    pub search_limit: usize,
}

impl Default for LcrioConfig {
    fn default() -> Self {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
        Self {
            panamax_root: PathBuf::from(&home).join("crates.io/full"),
            cache_dir: PathBuf::from(&home).join(".cache/lcrio"),
            search_limit: 20,
        }
    }
}

impl LcrioConfig {
    pub fn index_root(&self) -> PathBuf {
        self.panamax_root.join("crates.io-index")
    }

    pub fn crates_root(&self) -> PathBuf {
        self.panamax_root.join("crates")
    }
}
