use std::path::PathBuf;

/// Compute the panamax index prefix for a crate name.
///
/// - 1 char  → `1/`
/// - 2 chars → `2/`
/// - 3 chars → `3/{first_char}/`
/// - 4+ chars → `{first_two}/{next_two}/`
pub fn prefix(name: &str) -> String {
    let lower = name.to_ascii_lowercase();
    match lower.len() {
        0 => String::new(),
        1 => "1".to_string(),
        2 => "2".to_string(),
        3 => format!("3/{}", &lower[..1]),
        _ => format!("{}/{}", &lower[..2], &lower[2..4]),
    }
}

/// Resolve the path to a crate's index file.
pub fn index_path(index_root: &std::path::Path, name: &str) -> PathBuf {
    let lower = name.to_ascii_lowercase();
    index_root.join(prefix(&lower)).join(&lower)
}

/// Resolve the path to a .crate file.
pub fn crate_file_path(crates_root: &std::path::Path, name: &str, version: &str) -> PathBuf {
    let lower = name.to_ascii_lowercase();
    crates_root
        .join(prefix(&lower))
        .join(&lower)
        .join(version)
        .join(format!("{}-{}.crate", lower, version))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prefix_lengths() {
        assert_eq!(prefix("a"), "1");
        assert_eq!(prefix("ab"), "2");
        assert_eq!(prefix("tok"), "3/t");
        assert_eq!(prefix("serde"), "se/rd");
        assert_eq!(prefix("tokio"), "to/ki");
    }
}
