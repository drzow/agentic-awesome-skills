use anyhow::{anyhow, Result};
use chrono::Duration;
use std::path::Path;

use crate::cache::manifest::CacheManifest;
use crate::cache::eviction;

pub fn run(base_dir: &Path, command: &str, older_than_days: Option<u64>) -> Result<()> {
    let cache_dir = base_dir.join("cache");
    let manifest_path = cache_dir.join("manifest.json");

    match command {
        "info" => {
            if !manifest_path.exists() {
                println!("Cache is empty (no manifest found).");
                return Ok(());
            }
            let manifest = CacheManifest::load(&manifest_path)?;
            let stats = manifest.stats();
            println!("Cache statistics:");
            println!("  Entries: {}", stats.entry_count);
            println!("  Total accesses: {}", stats.total_accesses);
            if let Some(oldest) = stats.oldest_entry {
                println!("  Oldest entry: {}", oldest.format("%Y-%m-%d %H:%M:%S UTC"));
            }
            if let Some(newest) = stats.newest_entry {
                println!("  Newest entry: {}", newest.format("%Y-%m-%d %H:%M:%S UTC"));
            }
            println!("  Max size: {} MB", stats.max_size_mb);
        }
        "clear" => {
            if manifest_path.exists() {
                let mut manifest = CacheManifest::load(&manifest_path)?;
                manifest.clear();
                manifest.save(&manifest_path)?;
                // Remove cached skill directories
                if cache_dir.is_dir() {
                    for entry in std::fs::read_dir(&cache_dir).ok().into_iter().flatten() {
                        let entry = entry?;
                        let path = entry.path();
                        if path.is_dir() && path.file_name().map(|n| n != "manifest.json").unwrap_or(false) {
                            let _ = std::fs::remove_dir_all(&path);
                        }
                    }
                }
            }
            println!("Cache cleared.");
        }
        "prune" => {
            if !manifest_path.exists() {
                println!("Cache is empty (no manifest found).");
                return Ok(());
            }
            let older_than = match older_than_days {
                Some(days) => Duration::days(i64::try_from(days).unwrap_or(30)),
                None => Duration::days(30),
            };
            let mut manifest = CacheManifest::load(&manifest_path)?;
            let pruned = eviction::prune_old(&mut manifest, &cache_dir, older_than)?;
            manifest.save(&manifest_path)?;
            println!("Pruned {} entries older than {} days.", pruned, older_than.num_days());
        }
        _ => return Err(anyhow!("unknown cache command: {}", command)),
    }

    Ok(())
}
