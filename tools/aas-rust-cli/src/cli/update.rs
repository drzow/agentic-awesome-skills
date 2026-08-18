use anyhow::{anyhow, Result};
use std::fs;
use std::path::Path;

use crate::index::generator;
use crate::store::bare_repo::BareStore;
use crate::utils::atomic_write;

/// Update: fetch from origin and rebuild index if changed.
pub fn update(base_dir: &Path, dry_run: bool, skip_tls_verify: bool) -> Result<()> {
    let store_path = base_dir.join("store");
    let index_path = base_dir.join("index.json");
    let meta_path = base_dir.join("meta/state.json");

    if !store_path.exists() {
        return Err(anyhow!("store not found. Run 'aas init' first."));
    }

    let store = BareStore::open(&store_path)?;

    if meta_path.exists() {
        if let Ok(content) = fs::read_to_string(&meta_path) {
            if let Ok(state) = serde_json::from_str::<crate::models::CloneState>(&content) {
                store.ensure_origin(&state.repo_url)?;
            }
        }
    }

    // Fetch latest from origin
    println!("Fetching from origin...");
    let new_sha = match store.fetch(skip_tls_verify) {
        Ok(sha) => sha,
        Err(e) => {
            eprintln!("Warning: fetch failed: {}", e);
            eprintln!("Using local state.");
            return Ok(());
        }
    };

    // Read existing state
    let old_sha = if meta_path.exists() {
        let content = fs::read_to_string(&meta_path)?;
        let state: crate::models::CloneState = serde_json::from_str(&content)?;
        state.source_sha
    } else {
        store.head_sha()?
    };

    if new_sha == old_sha {
        println!("Already up to date (SHA: {}).", new_sha);
        return Ok(());
    }

    println!("New SHA: {}, current: {}", new_sha, old_sha);

    if dry_run {
        println!("[dry-run] Would update to {} and rebuild index.", new_sha);
        return Ok(());
    }

    // Update ref
    store.update_ref(&new_sha)?;

    let old_skill_count = if index_path.exists() {
        crate::index::reader::load_index(&index_path)
            .map(|index| index.skill_count)
            .unwrap_or(0)
    } else {
        0
    };

    // Regenerate index
    println!("Rebuilding index...");
    let new_index = generator::generate_index(&store)?;

    // Atomically replace index
    let tmp_path = index_path.with_extension("json.tmp");
    atomic_write::atomic_write(&tmp_path, serde_json::to_string_pretty(&new_index)?.as_bytes())?;
    fs::rename(&tmp_path, &index_path)?;

    // Update state
    if meta_path.exists() {
        let content = fs::read_to_string(&meta_path)?;
        let mut state: crate::models::CloneState = serde_json::from_str(&content)?;
        state.source_sha = new_sha;
        state.last_updated = chrono::Utc::now();
        if let Some(version) = new_index.catalog_version.clone() {
            state.version = version;
        }
        let tmp_meta = meta_path.with_extension("json.tmp");
        atomic_write::atomic_write(&tmp_meta, serde_json::to_string_pretty(&state)?.as_bytes())?;
        fs::rename(&tmp_meta, &meta_path)?;
    }

    println!("Updated: {} skills (was {})", new_index.skill_count, old_skill_count);
    Ok(())
}
