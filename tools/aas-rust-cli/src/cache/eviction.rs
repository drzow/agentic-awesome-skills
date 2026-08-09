use anyhow::Result;
use chrono::{Duration, Utc};
use std::fs;
use std::path::Path;

use super::manifest::CacheManifest;

/// Evict entries from the cache based on LRU policy (by access_count then fetched_at).
#[allow(dead_code)] // TODO(#1): implement in CLI cache commands or MCP filter tool
pub fn evict_lru(manifest: &mut CacheManifest, max_entries: usize) -> Result<()> {
    while manifest.entries.len() > max_entries {
        let victim_id = find_lru_entry(manifest);
        match victim_id {
            Some(id) => {
                manifest.remove(&id);
            }
            None => break, // No more entries to evict
        }
    }
    Ok(())
}

/// Find the least-recently-used entry ID.
#[allow(dead_code)] // Helper for evict_lru; used when LRU eviction is implemented
fn find_lru_entry(manifest: &CacheManifest) -> Option<String> {
    manifest.entries.iter()
        .min_by(|(_, a), (_, b)| {
            a.access_count.cmp(&b.access_count)
                .then_with(|| a.fetched_at.cmp(&b.fetched_at))
        })
        .map(|(id, _)| id.clone())
}

/// Prune entries older than the given duration. Returns count of pruned entries.
pub fn prune_old(manifest: &mut CacheManifest, cache_dir: &Path, older_than: Duration) -> Result<usize> {
    let cutoff = Utc::now() - older_than;
    let mut pruned = 0;

    let to_remove: Vec<String> = manifest.entries.iter()
        .filter(|(_, entry)| entry.fetched_at < cutoff)
        .map(|(id, _)| id.clone())
        .collect();

    for id in &to_remove {
        let entry_path = cache_dir.join(id);
        if entry_path.is_dir() && manifest.entries.contains_key(id.as_str()) {
            let _ = fs::remove_dir_all(&entry_path);
        }
        manifest.remove(id);
        pruned += 1;
    }

    Ok(pruned)
}
