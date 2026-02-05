use anyhow::Result;
use rmcp::{
    ServerHandler, ServiceExt,
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{CallToolResult, Content, ServerCapabilities, ServerInfo},
    schemars, tool, tool_handler, tool_router,
    transport::stdio,
};
use serde::Deserialize;
use tracing_subscriber::EnvFilter;

use lcrio::Lcrio;

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct SearchCratesRequest {
    #[schemars(description = "Search query (crate name, partial name, or keyword)")]
    pub query: String,

    #[schemars(description = "Search for crates that depend on this crate instead of name search")]
    pub by_dep: Option<bool>,

    #[schemars(description = "Search for crates with this feature name instead of name search")]
    pub by_feature: Option<bool>,

    #[schemars(description = "Maximum number of results (default: 20)")]
    pub limit: Option<usize>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct CrateInfoRequest {
    #[schemars(description = "Name of the crate")]
    pub name: String,

    #[schemars(description = "Specific version (default: latest non-yanked)")]
    pub version: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ListCrateFilesRequest {
    #[schemars(description = "Name of the crate")]
    pub name: String,

    #[schemars(description = "Specific version (default: latest non-yanked)")]
    pub version: Option<String>,

    #[schemars(description = "Glob pattern to filter files (e.g. 'src/**/*.rs', '*.toml')")]
    pub pattern: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ReadCrateFileRequest {
    #[schemars(description = "Name of the crate")]
    pub name: String,

    #[schemars(description = "Path relative to crate root (e.g. 'src/lib.rs')")]
    pub path: String,

    #[schemars(description = "Specific version (default: latest non-yanked)")]
    pub version: Option<String>,
}

#[derive(Clone)]
pub struct LcrioMcpServer {
    lcrio: &'static Lcrio,
    tool_router: ToolRouter<Self>,
}

impl LcrioMcpServer {
    pub fn new() -> Self {
        // Leak a Box to get 'static lifetime - server runs for process lifetime anyway
        let lcrio = Box::leak(Box::new(Lcrio::with_defaults()));
        Self {
            lcrio,
            tool_router: Self::tool_router(),
        }
    }

    fn resolve_version(&self, name: &str, version: Option<&str>) -> Result<String, String> {
        match version {
            Some(v) => Ok(v.to_string()),
            None => self
                .lcrio
                .latest_version(name)
                .map(|e| e.vers)
                .map_err(|e| format!("{:#}", e)),
        }
    }
}

#[tool_router]
impl LcrioMcpServer {
    #[tool(description = "Search for Rust crates by name (exact, prefix, contains, fuzzy matching), or search for crates that depend on a given crate, or crates with a specific feature. Uses a local panamax mirror of crates.io.")]
    fn search_crates(
        &self,
        Parameters(req): Parameters<SearchCratesRequest>,
    ) -> Result<CallToolResult, rmcp::model::ErrorData> {
        let limit = req.limit.unwrap_or(20);

        if req.by_dep.unwrap_or(false) {
            let results = self.lcrio.search_by_dep(&req.query, Some(limit));
            let text = if results.is_empty() {
                format!("No crates found that depend on '{}'", req.query)
            } else {
                let mut out = format!("Crates depending on '{}' ({} results):\n", req.query, results.len());
                for name in &results {
                    out.push_str(&format!("  {}\n", name));
                }
                out
            };
            Ok(CallToolResult::success(vec![Content::text(text)]))
        } else if req.by_feature.unwrap_or(false) {
            let results = self.lcrio.search_by_feature(&req.query, Some(limit));
            let text = if results.is_empty() {
                format!("No crates found with feature '{}'", req.query)
            } else {
                let mut out = format!("Crates with feature '{}' ({} results):\n", req.query, results.len());
                for name in &results {
                    out.push_str(&format!("  {}\n", name));
                }
                out
            };
            Ok(CallToolResult::success(vec![Content::text(text)]))
        } else {
            let results = self.lcrio.search(&req.query, Some(limit));
            let text = if results.is_empty() {
                format!("No crates found matching '{}'", req.query)
            } else {
                let mut out = format!("Search results for '{}' ({} results):\n", req.query, results.len());
                for r in &results {
                    out.push_str(&format!("  {:<40} {:.2}  ({})\n", r.name, r.score, r.match_type));
                }
                out
            };
            Ok(CallToolResult::success(vec![Content::text(text)]))
        }
    }

    #[tool(description = "Get metadata for a specific Rust crate: versions, dependencies, and features. Uses a local panamax mirror of crates.io.")]
    fn crate_info(
        &self,
        Parameters(req): Parameters<CrateInfoRequest>,
    ) -> Result<CallToolResult, rmcp::model::ErrorData> {
        let entries = match self.lcrio.crate_index(&req.name) {
            Ok(e) => e,
            Err(e) => {
                return Ok(CallToolResult::error(vec![Content::text(format!(
                    "Error reading index for '{}': {:#}",
                    req.name, e
                ))]));
            }
        };

        let entry = if let Some(ref ver) = req.version {
            match entries.iter().find(|e| e.vers == *ver) {
                Some(e) => e.clone(),
                None => {
                    return Ok(CallToolResult::error(vec![Content::text(format!(
                        "Version {} not found for '{}'",
                        ver, req.name
                    ))]));
                }
            }
        } else {
            match entries.iter().rev().find(|e| !e.yanked) {
                Some(e) => e.clone(),
                None => {
                    return Ok(CallToolResult::error(vec![Content::text(format!(
                        "No non-yanked versions found for '{}'",
                        req.name
                    ))]));
                }
            }
        };

        let mut out = format!("{} v{}\n", entry.name, entry.vers);
        if let Some(ref date) = entry.pubtime {
            out.push_str(&format!("Published: {}\n", date));
        }
        out.push_str(&format!("Total versions: {}\n", entries.len()));

        // Dependencies
        let normal: Vec<_> = entry.deps.iter().filter(|d| d.kind == "normal").collect();
        let dev: Vec<_> = entry.deps.iter().filter(|d| d.kind == "dev").collect();
        let build: Vec<_> = entry.deps.iter().filter(|d| d.kind == "build").collect();

        if !normal.is_empty() {
            out.push_str("\n[dependencies]\n");
            for dep in &normal {
                let opt = if dep.optional { " (optional)" } else { "" };
                out.push_str(&format!("  {} {}{}\n", dep.name, dep.req, opt));
            }
        }
        if !dev.is_empty() {
            out.push_str("\n[dev-dependencies]\n");
            for dep in &dev {
                out.push_str(&format!("  {} {}\n", dep.name, dep.req));
            }
        }
        if !build.is_empty() {
            out.push_str("\n[build-dependencies]\n");
            for dep in &build {
                out.push_str(&format!("  {} {}\n", dep.name, dep.req));
            }
        }

        // Features
        let features = entry.all_features();
        if !features.is_empty() {
            out.push_str("\n[features]\n");
            for (name, activates) in &features {
                if activates.is_empty() {
                    out.push_str(&format!("  {}\n", name));
                } else {
                    out.push_str(&format!("  {} = {:?}\n", name, activates));
                }
            }
        }

        // Recent versions
        out.push_str("\nRecent versions:\n");
        let start = entries.len().saturating_sub(10);
        for e in &entries[start..] {
            let yanked = if e.yanked { " (yanked)" } else { "" };
            let date = e.pubtime.as_deref().unwrap_or("unknown");
            out.push_str(&format!("  {:<14} {}{}\n", e.vers, date, yanked));
        }

        Ok(CallToolResult::success(vec![Content::text(out)]))
    }

    #[tool(description = "List files in an extracted Rust crate with optional glob pattern filter. Automatically extracts the crate if not cached. Uses a local panamax mirror.")]
    fn list_crate_files(
        &self,
        Parameters(req): Parameters<ListCrateFilesRequest>,
    ) -> Result<CallToolResult, rmcp::model::ErrorData> {
        let version = match self.resolve_version(&req.name, req.version.as_deref()) {
            Ok(v) => v,
            Err(e) => return Ok(CallToolResult::error(vec![Content::text(e)])),
        };

        match self.lcrio.list_files(&req.name, &version, req.pattern.as_deref()) {
            Ok(files) => {
                let mut out = format!("{} v{} files", req.name, version);
                if let Some(ref pat) = req.pattern {
                    out.push_str(&format!(" (pattern: {})", pat));
                }
                out.push_str(&format!(" ({} entries):\n", files.len()));
                for f in &files {
                    if f.is_dir {
                        out.push_str(&format!("  {}/\n", f.relative_path));
                    } else {
                        out.push_str(&format!("  {:<60} {:>8}\n", f.relative_path, format_size(f.size)));
                    }
                }
                Ok(CallToolResult::success(vec![Content::text(out)]))
            }
            Err(e) => Ok(CallToolResult::error(vec![Content::text(format!(
                "Error listing files for '{}' v{}: {:#}",
                req.name, version, e
            ))])),
        }
    }

    #[tool(description = "Read a specific file from an extracted Rust crate. Automatically extracts the crate if not cached. Uses a local panamax mirror.")]
    fn read_crate_file(
        &self,
        Parameters(req): Parameters<ReadCrateFileRequest>,
    ) -> Result<CallToolResult, rmcp::model::ErrorData> {
        let version = match self.resolve_version(&req.name, req.version.as_deref()) {
            Ok(v) => v,
            Err(e) => return Ok(CallToolResult::error(vec![Content::text(e)])),
        };

        match self.lcrio.read_file(&req.name, &version, &req.path) {
            Ok(content) => Ok(CallToolResult::success(vec![Content::text(content)])),
            Err(e) => Ok(CallToolResult::error(vec![Content::text(format!(
                "Error reading '{}' from '{}' v{}: {:#}",
                req.path, req.name, version, e
            ))])),
        }
    }
}

#[tool_handler]
impl ServerHandler for LcrioMcpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some(
                "MCP server for searching and browsing Rust crates from a local panamax \
                 mirror of crates.io. Provides tools to search crates, view metadata, \
                 list files, and read source code."
                    .to_string(),
            ),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
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

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .with_env_filter(EnvFilter::from_default_env())
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .init();

    tracing::info!("Starting lcrio MCP server");

    let server = LcrioMcpServer::new();
    let service = server.serve(stdio()).await.inspect_err(|e| {
        tracing::error!("serving error: {:?}", e);
    })?;

    service.waiting().await?;
    Ok(())
}
