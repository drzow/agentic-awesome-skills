use anyhow::{anyhow, Result};
use std::fs;
use std::path::Path;

use crate::index::generator;
use crate::store::bare_repo::BareStore;
use crate::utils::atomic_write;

/// Initialize: clone bare repo and generate initial index.
pub fn init(repo_url: &str, base_dir: &Path, force: bool) -> Result<()> {
    let store_path = base_dir.join("store");
    let index_path = base_dir.join("index.json");
    let meta_dir = base_dir.join("meta");

    if force && store_path.exists() {
        fs::remove_dir_all(&store_path)?;
    }

    // Clone bare repo
    println!("Cloning repository into {:?}...", store_path);
    let store_result = if store_path.exists() {
        BareStore::open(&store_path)
            .map_err(|e| anyhow!("existing store is not a valid git repo: {}", e))
    } else {
        BareStore::init(repo_url, &store_path)
    };
    let store = store_result?;

    // Generate index from git objects
    println!("Generating catalog index...");
    let index = generator::generate_index(&store)?;
    println!("Index generated with {} skills.", index.skill_count);

    // Write index atomically
    atomic_write::atomic_write(&index_path, serde_json::to_string_pretty(&index)?.as_bytes())?;

    // Write meta state
    let sha = store.head_sha()?;
    let version = index
        .catalog_version
        .clone()
        .unwrap_or_else(|| "unknown".to_string());
    let state = crate::models::CloneState {
        repo_url: repo_url.to_string(),
        source_sha: sha.clone(),
        cloned_at: chrono::Utc::now(),
        last_updated: chrono::Utc::now(),
        version: version.clone(),
    };
    fs::create_dir_all(&meta_dir)?;
    let meta_path = meta_dir.join("state.json");
    atomic_write::atomic_write(&meta_path, serde_json::to_string_pretty(&state)?.as_bytes())?;

    // Create cache dir
    fs::create_dir_all(base_dir.join("cache"))?;

    println!("Initialized AAS store at {:?}", base_dir);
    println!("  Store: {:?}", store_path);
    println!("  Index: {} skills", index.skill_count);
    println!("  Version: {}", version);
    println!("  SHA: {}", sha);

    Ok(())
}
