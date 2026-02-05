use anyhow::Result;
use clap::{Parser, Subcommand};
use lcrio::Lcrio;

#[derive(Parser)]
#[command(name = "lcrio", about = "Search and browse crates from a local panamax mirror")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Search for crates by name
    Search {
        query: String,
        /// Search for crates that depend on this crate instead
        #[arg(long)]
        dep: bool,
        /// Search for crates with this feature instead
        #[arg(long)]
        feature: bool,
        /// Maximum results
        #[arg(long, short, default_value = "20")]
        limit: usize,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Show crate versions and metadata
    Info {
        #[arg(rename_all = "verbatim")]
        krate: String,
        /// Show only the latest version
        #[arg(long)]
        latest: bool,
    },
    /// List dependencies of a crate
    Deps {
        #[arg(rename_all = "verbatim")]
        krate: String,
        /// Specific version (default: latest)
        version: Option<String>,
    },
    /// List features of a crate
    Features {
        #[arg(rename_all = "verbatim")]
        krate: String,
        /// Specific version (default: latest)
        version: Option<String>,
    },
    /// Extract a crate and print the path
    Unpack {
        #[arg(rename_all = "verbatim")]
        krate: String,
        /// Specific version (default: latest)
        version: Option<String>,
    },
    /// List files in a crate
    Ls {
        #[arg(rename_all = "verbatim")]
        krate: String,
        /// Specific version (default: latest)
        version: Option<String>,
        /// Glob pattern filter (e.g. "src/**/*.rs")
        #[arg(long, short)]
        pattern: Option<String>,
    },
    /// Read a file from a crate
    Cat {
        #[arg(rename_all = "verbatim")]
        krate: String,
        /// File path relative to crate root
        path: String,
        /// Specific version (default: latest)
        version: Option<String>,
    },
    /// Purge the extraction cache
    Clean,
}

fn resolve_version(lcrio: &Lcrio, name: &str, version: Option<&str>) -> Result<String> {
    match version {
        Some(v) => Ok(v.to_string()),
        None => Ok(lcrio.latest_version(name)?.vers),
    }
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let lcrio = Lcrio::with_defaults();

    match cli.command {
        Commands::Search { query, dep, feature, limit, json } => {
            if dep {
                let results = lcrio.search_by_dep(&query, Some(limit));
                if json {
                    println!("{}", serde_json::to_string_pretty(&results)?);
                } else {
                    for name in &results {
                        println!("{}", name);
                    }
                    println!("\n{} crates depend on '{}'", results.len(), query);
                }
            } else if feature {
                let results = lcrio.search_by_feature(&query, Some(limit));
                if json {
                    println!("{}", serde_json::to_string_pretty(&results)?);
                } else {
                    for name in &results {
                        println!("{}", name);
                    }
                    println!("\n{} crates have feature '{}'", results.len(), query);
                }
            } else {
                let results = lcrio.search(&query, Some(limit));
                if json {
                    let json_results: Vec<serde_json::Value> = results
                        .iter()
                        .map(|r| serde_json::json!({
                            "name": r.name,
                            "score": r.score,
                            "match_type": r.match_type.to_string(),
                        }))
                        .collect();
                    println!("{}", serde_json::to_string_pretty(&json_results)?);
                } else {
                    for r in &results {
                        println!("{:<40} {:.2}  ({})", r.name, r.score, r.match_type);
                    }
                }
            }
        }
        Commands::Info { krate, latest } => {
            if latest {
                let entry = lcrio.latest_version(&krate)?;
                print_entry(&entry);
            } else {
                let entries = lcrio.crate_index(&krate)?;
                println!("{}: {} versions", krate, entries.len());
                println!();
                // Show last 10 versions
                let start = entries.len().saturating_sub(10);
                for entry in &entries[start..] {
                    let yanked = if entry.yanked { " (yanked)" } else { "" };
                    let date = entry.pubtime.as_deref().unwrap_or("unknown");
                    println!("  {:<14} {}{}", entry.vers, date, yanked);
                }
                if entries.len() > 10 {
                    println!("  ... and {} earlier versions", entries.len() - 10);
                }
            }
        }
        Commands::Deps { krate, version } => {
            let version = resolve_version(&lcrio, &krate, version.as_deref())?;
            let entries = lcrio.crate_index(&krate)?;
            let entry = entries
                .iter()
                .find(|e| e.vers == version)
                .ok_or_else(|| anyhow::anyhow!("version {} not found", version))?;

            println!("{} v{} dependencies:", krate, version);
            println!();
            let mut normal: Vec<_> = entry.deps.iter().filter(|d| d.kind == "normal").collect();
            let mut dev: Vec<_> = entry.deps.iter().filter(|d| d.kind == "dev").collect();
            let mut build: Vec<_> = entry.deps.iter().filter(|d| d.kind == "build").collect();
            normal.sort_by_key(|d| &d.name);
            dev.sort_by_key(|d| &d.name);
            build.sort_by_key(|d| &d.name);

            if !normal.is_empty() {
                println!("[dependencies]");
                for dep in &normal {
                    let opt = if dep.optional { " (optional)" } else { "" };
                    println!("  {} {}{}", dep.name, dep.req, opt);
                }
            }
            if !dev.is_empty() {
                println!("\n[dev-dependencies]");
                for dep in &dev {
                    println!("  {} {}", dep.name, dep.req);
                }
            }
            if !build.is_empty() {
                println!("\n[build-dependencies]");
                for dep in &build {
                    println!("  {} {}", dep.name, dep.req);
                }
            }
        }
        Commands::Features { krate, version } => {
            let version = resolve_version(&lcrio, &krate, version.as_deref())?;
            let entries = lcrio.crate_index(&krate)?;
            let entry = entries
                .iter()
                .find(|e| e.vers == version)
                .ok_or_else(|| anyhow::anyhow!("version {} not found", version))?;

            let features = entry.all_features();
            println!("{} v{} features:", krate, version);
            println!();
            for (name, activates) in &features {
                if activates.is_empty() {
                    println!("  {}", name);
                } else {
                    println!("  {} = {:?}", name, activates);
                }
            }
        }
        Commands::Unpack { krate, version } => {
            let version = resolve_version(&lcrio, &krate, version.as_deref())?;
            let result = lcrio.unpack(&krate, &version)?;
            if result.was_cached {
                eprintln!("(cached)");
            }
            println!("{}", result.path.display());
        }
        Commands::Ls { krate, version, pattern } => {
            let version = resolve_version(&lcrio, &krate, version.as_deref())?;
            let files = lcrio.list_files(&krate, &version, pattern.as_deref())?;
            for f in &files {
                if f.is_dir {
                    println!("{}/ ", f.relative_path);
                } else {
                    println!("{:<60} {:>8}", f.relative_path, format_size(f.size));
                }
            }
        }
        Commands::Cat { krate, path, version } => {
            let version = resolve_version(&lcrio, &krate, version.as_deref())?;
            let content = lcrio.read_file(&krate, &version, &path)?;
            print!("{}", content);
        }
        Commands::Clean => {
            lcrio.clean()?;
            println!("Cache cleaned.");
        }
    }

    Ok(())
}

fn print_entry(entry: &lcrio::index::IndexEntry) {
    println!("{} v{}", entry.name, entry.vers);
    if let Some(ref date) = entry.pubtime {
        println!("  published: {}", date);
    }
    if entry.yanked {
        println!("  YANKED");
    }

    let features = entry.all_features();
    if !features.is_empty() {
        println!("  features: {}", features.keys().cloned().collect::<Vec<_>>().join(", "));
    }

    let normal_deps: Vec<_> = entry.deps.iter().filter(|d| d.kind == "normal").collect();
    if !normal_deps.is_empty() {
        println!("  deps: {}", normal_deps.iter().map(|d| d.name.as_str()).collect::<Vec<_>>().join(", "));
    }
}

fn format_size(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{}B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1}K", bytes as f64 / 1024.0)
    } else {
        format!("{:.1}M", bytes as f64 / (1024.0 * 1024.0))
    }
}
