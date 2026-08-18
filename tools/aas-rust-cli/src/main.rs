use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::{Path, PathBuf};

mod cache;
mod cli;
mod index;
mod mcp;
mod models;
mod search;
mod store;
mod utils;

/// AAS — Minimal-context skill management for agentic skills.
#[derive(Parser, Debug)]
#[command(name = "aas", version, about, long_about = None)]
struct Cli {
    /// Base directory for AAS data (~/.aas by default).
    #[arg(short, long, default_value_t = String::from("~/.aas"))]
    base: String,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Clone bare repo and generate initial index.
    Init {
        /// Git repository URL to clone.
        #[arg(long)]
        repo: String,
        /// Force re-initialization (delete existing store).
        #[arg(long, default_value_t = false)]
        force: bool,
    },

    /// Fetch from origin and rebuild index if changed.
    Update {
        /// Don't actually update, just show what would change.
        #[arg(long, default_value_t = false)]
        dry_run: bool,
    },

    /// Show store info, index stats, and version.
    Status,

    /// Search skills by keyword with relevance scoring.
    Search {
        /// Search query string.
        query: String,
        /// Maximum number of results to return.
        #[arg(long, default_value_t = 10)]
        limit: usize,
    },

    /// List skills filtered by category, risk, or tags.
    List {
        /// Filter by category name.
        #[arg(long)]
        category: Option<String>,
        /// Filter by risk level (safe, none, moderate, critical).
        #[arg(long)]
        risk: Option<String>,
        /// Filter by tag (can be specified multiple times).
        #[arg(long)]
        tags: Vec<String>,
        /// Maximum number of results.
        #[arg(long, default_value_t = 50)]
        limit: usize,
    },

    /// List all categories with skill counts.
    Categories,

    /// Fetch and print the full SKILL.md for a skill.
    Get {
        /// Skill ID to fetch.
        skill_id: String,
    },

    /// Manage the skill content cache.
    Cache {
        #[command(subcommand)]
        subcommand: CacheCommand,
    },

    /// Activate skills (create symlinks/copy into agent directories).
    Activate {
        /// Skill IDs to activate.
        skill_ids: Vec<String>,
        /// Target agent directories (comma-separated): opencode,claude-code,cursor...
        #[arg(long, default_value = "")]
        targets: String,
    },

    /// Deactivate skills (remove symlinks/copies from agent directories).
    Deactivate {
        /// Skill IDs to deactivate.
        skill_ids: Vec<String>,
        /// Target agent directories (comma-separated).
        #[arg(long, default_value = "")]
        targets: String,
    },

    /// Start the MCP server in stdio mode.
    Mcp {
        /// Path to config file.
        #[arg(long)]
        config: Option<String>,
    },
}

#[derive(Subcommand, Debug)]
enum CacheCommand {
    /// Show cache statistics.
    Info,
    /// Clear all cached content.
    Clear,
    /// Remove entries not accessed in N days.
    Prune {
        /// Remove entries older than this many days.
        #[arg(long, default_value_t = 30)]
        older_than: u64,
    },
}

fn resolve_base(base: &str) -> PathBuf {
    let expanded = shellexpand::full(base).unwrap_or_else(|_| shellexpand::AllocatedString(base.to_string()));
    PathBuf::from(expanded.0.as_str())
}

mod shellexpand {
    pub fn full(s: &str) -> Result<AllocatedString, ()> {
        if let Some(rest) = s.strip_prefix("~/") {
            if let Some(home) = dirs::home_dir() {
                return Ok(AllocatedString(home.join(rest).display().to_string()));
            }
        }
        Err(())
    }

    pub struct AllocatedString(pub String);
    impl std::ops::Deref for AllocatedString {
        type Target = str;
        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let base_dir = resolve_base(&cli.base);

    match cli.command {
        Some(Commands::Init { repo, force }) => {
            cli::init::init(&repo, &base_dir, force)?;
        }
        Some(Commands::Update { dry_run }) => {
            cli::update::update(&base_dir, dry_run)?;
        }
        Some(Commands::Status) => {
            cli::status::run(&base_dir)?;
        }
        Some(Commands::Search { query, limit }) => {
            cli::search_cmd::run(&base_dir, &query, limit)?;
        }
        Some(Commands::List { category, risk, tags, limit }) => {
            cli::list::run(&base_dir, category.as_deref(), risk.as_deref(), tags, limit)?;
        }
        Some(Commands::Categories) => {
            let index_path = base_dir.join("index.json");
            if !index_path.exists() {
                eprintln!("Index not found. Run 'aas init' first.");
                std::process::exit(1);
            }
            let index = index::reader::load_index(&index_path)?;
            let categories = search::scoring::list_categories(&index);
            println!("{:<30} Count", "Category");
            println!("{}", "-".repeat(42));
            for (name, count) in &categories {
                println!("{:<30} {}", name, count);
            }
            println!("\n{} total categories.", categories.len());
        }
        Some(Commands::Get { skill_id }) => {
            cli::get_skill::run(&base_dir, &skill_id)?;
        }
        Some(Commands::Cache { subcommand }) => {
            let older_than = match &subcommand {
                CacheCommand::Prune { older_than } => Some(*older_than),
                _ => None,
            };
            let name = match &subcommand {
                CacheCommand::Info => "info",
                CacheCommand::Clear => "clear",
                CacheCommand::Prune { .. } => "prune",
            };
            cli::cache::run(&base_dir, name, older_than)?;
        }
        Some(Commands::Activate { skill_ids, targets }) => {
            let target_names = parse_targets(&targets);
            cli::activate::run(&base_dir, "activate", &skill_ids, &target_names)?;
        }
        Some(Commands::Deactivate { skill_ids, targets }) => {
            let target_names = parse_targets(&targets);
            cli::activate::run(&base_dir, "deactivate", &skill_ids, &target_names)?;
        }
        Some(Commands::Mcp { .. }) => {
            mcp_server(&base_dir)?;
        }
        None => {
            println!("AAS - Minimal-context skill management.");
            println!("Run with --help for usage information.");
            println!();
            println!("Available commands:");
            println!("  init       Clone bare repo and generate index");
            println!("  update     Fetch from origin and rebuild index");
            println!("  status     Show store info and stats");
            println!("  search     Search skills by keyword");
            println!("  list       List skills with filters");
            println!("  categories List all categories");
            println!("  get        Get full SKILL.md content");
            println!("  cache      Manage skill cache");
            println!("  activate   Activate skills for agent directories");
            println!("  deactivate Deactivate skills");
            println!("  mcp        Start MCP server (stdio)");
        }
    }

    Ok(())
}

fn parse_targets(targets: &str) -> Vec<String> {
    if targets.is_empty() {
        return Vec::new();
    }
    targets.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect()
}

fn mcp_server(base_dir: &Path) -> Result<()> {
    use crate::mcp::tools::McpServer;

    let server = McpServer::new(base_dir)
        .map_err(|e| anyhow::anyhow!("{}", e))?;

    eprintln!("AAS MCP server starting on stdio...");
    mcp::server::start_server(Box::new(server));
    Ok(())
}
