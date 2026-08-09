use anyhow::{anyhow, Result};
use std::fs;
use std::path::PathBuf;

use crate::index::generator;
use crate::store::bare_repo::BareStore;
use crate::utils::atomic_write;

/// Update: fetch from origin and rebuild index if changed.
pub fn update(base_dir: &PathBuf, dry_run: bool) -> Result<()> {
    let store_path = base_dir.join("store");
    let index_path = base_dir.join("index.json");
    let meta_path = base_dir.join("meta/state.json");

    if !store_path.exists() {
        return Err(anyhow!("store not found. Run 'aas init' first."));
    }

    let store = BareStore::open(&store_path)?;

    // Fetch latest from origin
    println!("Fetching from origin...");
    let new_sha = match store.fetch() {
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
        let tmp_meta = meta_path.with_extension("json.tmp");
        atomic_write::atomic_write(&tmp_meta, serde_json::to_string_pretty(&state)?.as_bytes())?;
        fs::rename(&tmp_meta, &meta_path)?;
    }

    println!("Updated: {} skills (was {})", new_index.skill_count, state_old_skill_count(&old_sha));
    Ok(())
}

fn state_old_skill_count(_sha: &str) -> usize {
    0 // We don't track this separately; the user can run 'aas status' to see counts
}
