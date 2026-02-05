use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

pub struct UnpackResult {
    pub path: PathBuf,
    pub was_cached: bool,
}

/// Get the cache directory for an unpacked crate.
fn cache_path(cache_dir: &Path, name: &str, version: &str) -> PathBuf {
    cache_dir
        .join("unpacked")
        .join(name)
        .join(version)
}

/// Unpack a .crate file, returning the path to the extracted directory.
/// Uses cache to avoid re-extracting.
pub fn unpack_crate(
    crates_root: &Path,
    cache_dir: &Path,
    name: &str,
    version: &str,
) -> Result<UnpackResult> {
    let dest = cache_path(cache_dir, name, version);
    let inner_dir = dest.join(format!("{}-{}", name, version));

    // Check cache: directory exists with a Cargo.toml
    if inner_dir.join("Cargo.toml").exists() {
        return Ok(UnpackResult {
            path: inner_dir,
            was_cached: true,
        });
    }

    // Find the .crate file
    let crate_file = crate::prefix::crate_file_path(crates_root, name, version);
    if !crate_file.exists() {
        anyhow::bail!(
            "crate file not found: {} (expected at {})",
            format!("{}-{}.crate", name, version),
            crate_file.display()
        );
    }

    // Create destination directory
    std::fs::create_dir_all(&dest)
        .with_context(|| format!("creating cache dir {}", dest.display()))?;

    // Extract: .crate is a gzipped tar
    let file = std::fs::File::open(&crate_file)
        .with_context(|| format!("opening {}", crate_file.display()))?;
    let gz = flate2::read::GzDecoder::new(file);
    let mut archive = tar::Archive::new(gz);
    archive
        .unpack(&dest)
        .with_context(|| format!("extracting {}", crate_file.display()))?;

    if !inner_dir.exists() {
        // Some crates might extract with a different directory name; try to find it
        if let Ok(entries) = std::fs::read_dir(&dest) {
            for entry in entries.flatten() {
                if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                    return Ok(UnpackResult {
                        path: entry.path(),
                        was_cached: false,
                    });
                }
            }
        }
        anyhow::bail!("extraction succeeded but inner directory not found at {}", inner_dir.display());
    }

    Ok(UnpackResult {
        path: inner_dir,
        was_cached: false,
    })
}

/// Unpack the latest version of a crate.
pub fn unpack_latest(
    index_root: &Path,
    crates_root: &Path,
    cache_dir: &Path,
    name: &str,
) -> Result<(UnpackResult, String)> {
    let latest = crate::index::read_latest_version(index_root, name)?;
    let result = unpack_crate(crates_root, cache_dir, name, &latest.vers)?;
    Ok((result, latest.vers))
}

/// Return the pre-extracted source path from cargo registry src directory.
/// In cargo mode, sources are already extracted at `src_root/{name}-{version}/`.
pub fn cargo_source_path(
    src_root: &Path,
    name: &str,
    version: &str,
) -> Result<UnpackResult> {
    let dir = src_root.join(format!("{}-{}", name, version));
    if dir.exists() {
        Ok(UnpackResult {
            path: dir,
            was_cached: true,
        })
    } else {
        anyhow::bail!(
            "source not found at {} — try running `cargo fetch` in your project",
            dir.display()
        )
    }
}

/// Remove the entire unpacked cache.
pub fn clean_cache(cache_dir: &Path) -> Result<()> {
    let unpacked = cache_dir.join("unpacked");
    if unpacked.exists() {
        std::fs::remove_dir_all(&unpacked)
            .with_context(|| format!("removing {}", unpacked.display()))?;
    }
    Ok(())
}
