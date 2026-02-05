use std::path::Path;
use walkdir::WalkDir;

/// Build a sorted Vec of all crate names from the index directory.
pub fn build_name_index(index_root: &Path) -> Vec<String> {
    let mut names = Vec::with_capacity(250_000);
    for entry in WalkDir::new(index_root)
        .min_depth(2) // skip top-level dirs themselves
        .into_iter()
        .filter_entry(|e| {
            let name = e.file_name().to_string_lossy();
            name != ".git" && name != ".github" && name != "config.json"
        })
    {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };
        if entry.file_type().is_file() {
            let name = entry.file_name().to_string_lossy().to_string();
            // Skip config.json and hidden files
            if !name.starts_with('.') && name != "config.json" {
                names.push(name);
            }
        }
    }
    names.sort_unstable();
    names.dedup();
    names
}

#[derive(Debug, Clone)]
pub struct SearchResult {
    pub name: String,
    pub score: f64,
    pub match_type: MatchType,
}

#[derive(Debug, Clone, Copy)]
pub enum MatchType {
    Exact,
    Prefix,
    Contains,
    Fuzzy,
}

impl std::fmt::Display for MatchType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MatchType::Exact => write!(f, "exact"),
            MatchType::Prefix => write!(f, "prefix"),
            MatchType::Contains => write!(f, "contains"),
            MatchType::Fuzzy => write!(f, "fuzzy"),
        }
    }
}

/// Search for crate names matching the query.
pub fn search_names(names: &[String], query: &str, limit: usize) -> Vec<SearchResult> {
    let query_lower = query.to_ascii_lowercase();
    let mut results: Vec<SearchResult> = Vec::new();
    let mut seen = std::collections::HashSet::new();

    // 1. Exact match via binary search
    if let Ok(idx) = names.binary_search_by(|n| n.as_str().cmp(&query_lower)) {
        seen.insert(idx);
        results.push(SearchResult {
            name: names[idx].clone(),
            score: 1.0,
            match_type: MatchType::Exact,
        });
    }

    // 2. Prefix match via binary search + scan
    let start = names.partition_point(|n| n.as_str() < query_lower.as_str());
    for i in start..names.len() {
        if !names[i].starts_with(&query_lower) {
            break;
        }
        if seen.insert(i) {
            results.push(SearchResult {
                name: names[i].clone(),
                score: 0.9,
                match_type: MatchType::Prefix,
            });
        }
        if results.len() >= limit * 2 {
            break;
        }
    }

    // 3. Contains match
    if results.len() < limit {
        for (i, name) in names.iter().enumerate() {
            if name.contains(&query_lower) && seen.insert(i) {
                results.push(SearchResult {
                    name: name.clone(),
                    score: 0.7,
                    match_type: MatchType::Contains,
                });
                if results.len() >= limit * 2 {
                    break;
                }
            }
        }
    }

    // 4. Fuzzy match (Jaro-Winkler)
    if results.len() < limit {
        for (i, name) in names.iter().enumerate() {
            if seen.contains(&i) {
                continue;
            }
            let sim = strsim::jaro_winkler(&query_lower, name);
            if sim > 0.8 {
                seen.insert(i);
                results.push(SearchResult {
                    name: name.clone(),
                    score: sim * 0.6, // scale down to rank below contains
                    match_type: MatchType::Fuzzy,
                });
                if results.len() >= limit * 2 {
                    break;
                }
            }
        }
    }

    // Sort by score descending, truncate
    results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
    results.truncate(limit);
    results
}

/// Search for crates that depend on a given crate name.
/// This is expensive (scans all index files) so we limit the search.
pub fn search_by_dep(index_root: &Path, names: &[String], dep_name: &str, limit: usize) -> Vec<String> {
    let dep_lower = dep_name.to_ascii_lowercase();
    let mut results = Vec::new();

    for name in names {
        let path = crate::prefix::index_path(index_root, name);
        if let Ok(content) = std::fs::read_to_string(&path) {
            // Check last line (latest version) for the dep
            if let Some(last_line) = content.lines().rev().find(|l| !l.trim().is_empty()) {
                if let Ok(entry) = serde_json::from_str::<crate::index::IndexEntry>(last_line) {
                    if !entry.yanked && entry.deps.iter().any(|d| d.name.to_ascii_lowercase() == dep_lower) {
                        results.push(name.clone());
                        if results.len() >= limit {
                            break;
                        }
                    }
                }
            }
        }
    }
    results
}

/// Search for crates that have a given feature name.
pub fn search_by_feature(index_root: &Path, names: &[String], feature_name: &str, limit: usize) -> Vec<String> {
    let feat_lower = feature_name.to_ascii_lowercase();
    let mut results = Vec::new();

    for name in names {
        let path = crate::prefix::index_path(index_root, name);
        if let Ok(content) = std::fs::read_to_string(&path) {
            if let Some(last_line) = content.lines().rev().find(|l| !l.trim().is_empty()) {
                if let Ok(entry) = serde_json::from_str::<crate::index::IndexEntry>(last_line) {
                    if !entry.yanked {
                        let all = entry.all_features();
                        if all.keys().any(|k| k.to_ascii_lowercase() == feat_lower) {
                            results.push(name.clone());
                            if results.len() >= limit {
                                break;
                            }
                        }
                    }
                }
            }
        }
    }
    results
}
