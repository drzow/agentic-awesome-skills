use anyhow::{anyhow, Result};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Cache entry metadata.
#[allow(dead_code)] // Used as HashMap value type in CacheManifest::entries
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CacheEntry {
    pub skill_id: String,
    #[serde(rename = "contentSha256")]
    pub content_sha256: String,
    #[serde(rename = "fetchedAt")]
    pub fetched_at: chrono::DateTime<chrono::Utc>,
    #[serde(rename = "accessCount")]
    pub access_count: u64,
}

/// Persistent cache manifest with LRU metadata.
#[allow(dead_code)] // Used as type in mcp tools and CLI cache commands
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CacheManifest {
    pub schema_version: u32,
    #[serde(rename = "maxSizeMB")]
    pub max_size_mb: u64,
    pub entries: HashMap<String, CacheEntry>,
}

impl CacheManifest {
    /// Create a new empty manifest.
    #[allow(dead_code)] // Public API, constructed in tests and MCP tools
    pub fn new(max_size_mb: u64) -> Self {
        CacheManifest {
            schema_version: 1,
            max_size_mb,
            entries: HashMap::new(),
        }
    }

    /// Load manifest from disk.
    pub fn load(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| anyhow!("failed to read cache manifest at {:?}: {}", path, e))?;
        let manifest: CacheManifest = serde_json::from_str(&content)
            .map_err(|e| anyhow!("failed to parse cache manifest: {}", e))?;
        Ok(manifest)
    }

    /// Save manifest to disk atomically.
    pub fn save(&self, path: &Path) -> Result<()> {
        let content = serde_json::to_string_pretty(self)
            .map_err(|e| anyhow!("failed to serialize cache manifest: {}", e))?;
        crate::utils::atomic_write::atomic_write(path, content.as_bytes())?;
        Ok(())
    }

    /// Add or update a cache entry. Increments access count.
    pub fn record_access(&mut self, skill_id: &str, sha256: &str) {
        let now = Utc::now();
        if let Some(entry) = self.entries.get_mut(skill_id) {
            entry.access_count += 1;
            entry.fetched_at = now;
            entry.content_sha256 = sha256.to_string();
        } else {
            self.entries.insert(skill_id.to_string(), CacheEntry {
                skill_id: skill_id.to_string(),
                content_sha256: sha256.to_string(),
                fetched_at: now,
                access_count: 1,
            });
        }
    }

    /// Remove a cache entry. Returns true if it existed.
    pub fn remove(&mut self, skill_id: &str) -> bool {
        self.entries.remove(skill_id).is_some()
    }

    /// Clear all entries.
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// Get cache statistics.
    pub fn stats(&self) -> CacheStats {
        let total_accesses: u64 = self.entries.values().map(|e| e.access_count).sum();
        let oldest = self.entries.values()
            .min_by_key(|e| e.fetched_at)
            .map(|e| e.fetched_at);
        let newest = self.entries.values()
            .max_by_key(|e| e.fetched_at)
            .map(|e| e.fetched_at);

        CacheStats {
            entry_count: self.entries.len(),
            total_accesses,
            oldest_entry: oldest,
            newest_entry: newest,
            max_size_mb: self.max_size_mb,
        }
    }

    /// Get the content path for a skill in the cache.
    pub fn content_path(&self, base_path: &Path, skill_id: &str) -> PathBuf {
        base_path.join(skill_id).join("SKILL.md")
    }
}

/// Cache statistics summary.
#[derive(Debug)]
pub struct CacheStats {
    pub entry_count: usize,
    pub total_accesses: u64,
    pub oldest_entry: Option<chrono::DateTime<chrono::Utc>>,
    pub newest_entry: Option<chrono::DateTime<chrono::Utc>>,
    pub max_size_mb: u64,
}
