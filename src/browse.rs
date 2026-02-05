use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[derive(Debug, Clone)]
pub struct FileEntry {
    pub relative_path: String,
    pub size: u64,
    pub is_dir: bool,
}

/// List files in an extracted crate directory.
/// If `pattern` is provided, only files whose relative path contains the pattern are returned.
pub fn list_files(crate_root: &Path, pattern: Option<&str>) -> Result<Vec<FileEntry>> {
    if !crate_root.exists() {
        anyhow::bail!("crate directory does not exist: {}", crate_root.display());
    }

    let mut entries = Vec::new();
    for entry in WalkDir::new(crate_root).min_depth(1).sort_by_file_name() {
        let entry = entry.with_context(|| "walking crate directory")?;
        let relative = entry
            .path()
            .strip_prefix(crate_root)
            .unwrap_or(entry.path());
        let relative_str = relative.to_string_lossy().to_string();

        // Apply pattern filter
        if let Some(pat) = pattern {
            if !glob_match(&relative_str, pat) {
                continue;
            }
        }

        let metadata = entry.metadata().with_context(|| "reading file metadata")?;
        entries.push(FileEntry {
            relative_path: relative_str,
            size: metadata.len(),
            is_dir: metadata.is_dir(),
        });
    }
    Ok(entries)
}

/// Simple glob matching: supports `*` (any chars except `/`) and `**` (any path segment).
fn glob_match(path: &str, pattern: &str) -> bool {
    // Simple approach: convert glob to a contains check for basic patterns
    // For patterns like "src/**/*.rs", we do a two-part check
    if pattern.contains("**") {
        // Split on ** and check if parts match
        let parts: Vec<&str> = pattern.split("**").collect();
        if parts.len() == 2 {
            let prefix = parts[0].trim_end_matches('/');
            let suffix = parts[1].trim_start_matches('/');

            let prefix_ok = prefix.is_empty() || path.starts_with(prefix);
            let suffix_ok = if suffix.is_empty() {
                true
            } else if suffix.starts_with("*.") {
                let ext = &suffix[1..]; // e.g. ".rs"
                path.ends_with(ext)
            } else {
                path.contains(suffix)
            };
            return prefix_ok && suffix_ok;
        }
    }

    if pattern.starts_with("*.") {
        let ext = &pattern[1..];
        return path.ends_with(ext);
    }

    // Fallback: simple contains
    path.contains(pattern)
}

/// Read a file from an extracted crate.
pub fn read_file(crate_root: &Path, relative_path: &str) -> Result<String> {
    let full_path = crate_root.join(relative_path);

    // Security: ensure the path doesn't escape the crate root
    let canonical_root = crate_root.canonicalize()
        .with_context(|| format!("canonicalizing {}", crate_root.display()))?;
    let canonical_file = full_path.canonicalize()
        .with_context(|| format!("file not found: {}", relative_path))?;

    if !canonical_file.starts_with(&canonical_root) {
        anyhow::bail!("path traversal detected: {}", relative_path);
    }

    std::fs::read_to_string(&canonical_file)
        .with_context(|| format!("reading {}", relative_path))
}

/// Get the path to the crate root without reading files.
pub fn crate_root_path(cache_dir: &Path, name: &str, version: &str) -> PathBuf {
    cache_dir
        .join("unpacked")
        .join(name)
        .join(version)
        .join(format!("{}-{}", name, version))
}
