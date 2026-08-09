use anyhow::Result;
use std::fs;
use std::path::PathBuf;

/// Show store info, index stats, and version.
pub fn run(base_dir: &PathBuf) -> Result<()> {
    let store_path = base_dir.join("store");
    let index_path = base_dir.join("index.json");
    let meta_path = base_dir.join("meta/state.json");
    let cache_dir = base_dir.join("cache");

    println!("AAS Status");
    println!("{}", "=".repeat(50));

    // Store info
    if store_path.exists() {
        println!("\nStore:");
        println!("  Path: {:?}", store_path);
        if let Ok(store) = crate::store::bare_repo::BareStore::open(&store_path) {
            match store.head_sha() {
                Ok(sha) => println!("  HEAD: {}", sha),
                Err(_) => println!("  HEAD: unknown"),
            }
        }
    } else {
        println!("\nStore: not initialized");
    }

    // Index info
    if index_path.exists() {
        let index = crate::index::reader::load_index(&index_path)?;
        println!("\nIndex:");
        println!("  Skills: {}", index.skill_count);
        println!("  Digest: {}", index.catalog_digest);
        println!("  Generated: {}", index.generated_at.format("%Y-%m-%d %H:%M:%S UTC"));
    } else {
        println!("\nIndex: not generated");
    }

    // Meta info
    if meta_path.exists() {
        let content = fs::read_to_string(&meta_path)?;
        let state: crate::models::CloneState = serde_json::from_str(&content)?;
        println!("\nClone State:");
        println!("  URL: {}", state.repo_url);
        println!("  SHA: {}", state.source_sha);
        println!("  Version: {}", state.version);
        println!("  Last updated: {}", state.last_updated.format("%Y-%m-%d %H:%M:%S UTC"));
    }

    // Cache info
    let manifest_path = cache_dir.join("manifest.json");
    if manifest_path.exists() {
        let manifest = crate::cache::manifest::CacheManifest::load(&manifest_path)?;
        let stats = manifest.stats();
        println!("\nCache:");
        println!("  Entries: {}", stats.entry_count);
        println!("  Max size: {} MB", stats.max_size_mb);
    } else {
        println!("\nCache: empty");
    }

    Ok(())
}
