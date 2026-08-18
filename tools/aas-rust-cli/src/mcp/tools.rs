use serde_json::Value;

use crate::models::{CatalogIndex, CategoryCount};
use crate::index::reader;
use crate::search;
use crate::store::bare_repo::BareStore;
use crate::cache::manifest::CacheManifest as Manifest;
use crate::utils::path_validation::validate_skill_id;
use std::path::{Path, PathBuf};

/// MCP server that provides skill search and retrieval tools.
pub struct McpServer {
    pub index: CatalogIndex,
    pub store: BareStore,
    pub cache_manifest: Option<Manifest>,
    pub base_dir: PathBuf,
}

impl crate::mcp::server::McpHandler for McpServer {
    fn list_tools(&self) -> Vec<Value> {
        super::schema::tool_definitions()
    }

    fn handle_tool_call(&mut self, name: &str, args: Value) -> Result<Value, String> {
        match name {
            "search_skills" => self.tool_search(args),
            "get_skill" => self.tool_get_skill(args),
            "list_categories" => self.tool_list_categories(args),
            "filter_skills" => self.tool_filter_skills(args),
            _ => Err(format!("Unknown tool: {}", name)),
        }
    }
}

impl McpServer {
    /// Create a new MCP server from the given base directory.
    pub fn new(base_dir: &Path) -> Result<Self, String> {
        let store_path = base_dir.join("store");
        if !store_path.exists() {
            return Err("store not found. Run 'aas init' first.".to_string());
        }

        let store = BareStore::open(&store_path)
            .map_err(|e| format!("failed to open store: {}", e))?;

        let index_path = base_dir.join("index.json");
        let index = reader::load_index(&index_path)
            .map_err(|e| format!("failed to load index: {}", e))?;

        let cache_manifest = {
            let manifest_path = base_dir.join("cache/manifest.json");
            if manifest_path.exists() {
                Manifest::load(&manifest_path).ok()
            } else {
                None
            }
        };

        Ok(McpServer {
            index,
            store,
            cache_manifest,
            base_dir: base_dir.to_path_buf(),
        })
    }

    fn tool_search(&self, args: Value) -> Result<Value, String> {
        let query = args.get("query")
            .and_then(|v| v.as_str())
            .ok_or("missing required field 'query'")?;

        let limit = parse_limit(&args, 20, 50);

        let results = search::scoring::search(&self.index, query, limit);

        Ok(serde_json::json!({
            "ok": true,
            "results": results,
            "totalMatches": results.len(),
            "catalog": {
                "version": self.index.skill_count.to_string(),
                "digest": self.index.catalog_digest
            }
        }))
    }

    fn tool_get_skill(&mut self, args: Value) -> Result<Value, String> {
        let id = args.get("id")
            .and_then(|v| v.as_str())
            .ok_or("missing required field 'id'")?;

        validate_skill_id(id)?;

        let include_content = args.get("include_content")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);

        let skill_entry = self.index.skills.iter()
            .find(|s| s.id == id)
            .ok_or_else(|| format!("skill '{}' not found in index", id))?;

        if !include_content {
            return Ok(serde_json::json!({
                "ok": true,
                "skill": {
                    "id": skill_entry.id,
                    "name": skill_entry.name,
                    "description": skill_entry.description,
                    "category": skill_entry.category,
                    "tags": skill_entry.tags,
                    "risk": skill_entry.risk,
                },
                "content": null,
                "catalog": {
                    "version": self.index.skill_count.to_string(),
                    "digest": self.index.catalog_digest
                }
            }));
        }

        let content_bytes: Vec<u8> = self.store.get_blob_at_path(id)
            .map_err(|e| format!("failed to fetch skill content: {}", e))?;
        let content = String::from_utf8_lossy(&content_bytes).to_string();

        if let Some(ref mut manifest) = self.cache_manifest {
            let sha256 = compute_sha256(&content);
            manifest.record_access(id, &sha256);
            let _ = manifest.save(self.base_dir.join("cache/manifest.json").as_path());
            let cache_dir = self.base_dir.join("cache");
            let content_path = manifest.content_path(&cache_dir, id);
            let _ = std::fs::create_dir_all(content_path.parent().unwrap());
            let _ = std::fs::write(&content_path, &content);
        }

        Ok(serde_json::json!({
            "ok": true,
            "skill": {
                "id": skill_entry.id,
                "name": skill_entry.name,
                "description": skill_entry.description,
                "category": skill_entry.category,
                "tags": skill_entry.tags,
                "risk": skill_entry.risk,
            },
            "content": content,
            "catalog": {
                "version": self.index.skill_count.to_string(),
                "digest": self.index.catalog_digest
            }
        }))
    }

    fn tool_list_categories(&self, _args: Value) -> Result<Value, String> {
        let categories = search::scoring::list_categories(&self.index);
        let counts: Vec<CategoryCount> = categories.iter()
            .map(|(name, count)| CategoryCount {
                category: name.clone(),
                count: *count,
            })
            .collect();

        Ok(serde_json::json!({
            "ok": true,
            "categories": counts,
            "totalCategories": counts.len(),
            "catalog": {
                "version": self.index.skill_count.to_string(),
                "digest": self.index.catalog_digest
            }
        }))
    }

    fn tool_filter_skills(&self, args: Value) -> Result<Value, String> {
        let category = args.get("category").and_then(|v| v.as_str());
        let risk = args.get("risk").and_then(|v| v.as_str());

        let tags: Vec<&str> = args.get("tags")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str())
                    .collect()
            })
            .unwrap_or_default();

        let limit = parse_limit(&args, 50, 200);

        let results = search::scoring::filter(&self.index, category, risk, &tags, limit);

        Ok(serde_json::json!({
            "ok": true,
            "results": results,
            "totalMatches": results.len(),
            "catalog": {
                "version": self.index.skill_count.to_string(),
                "digest": self.index.catalog_digest
            }
        }))
    }
}

fn compute_sha256(content: &str) -> String {
    use sha2::{Sha256, Digest};
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    hex::encode(hasher.finalize())
}

fn parse_limit(args: &Value, default: u64, max: u64) -> usize {
    args.get("limit")
        .and_then(|v| v.as_u64())
        .unwrap_or(default)
        .min(max) as usize
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parse_limit_uses_default_when_missing() {
        assert_eq!(parse_limit(&json!({}), 20, 50), 20);
    }

    #[test]
    fn parse_limit_clamps_to_max() {
        assert_eq!(parse_limit(&json!({"limit": u64::MAX}), 20, 50), 50);
        assert_eq!(parse_limit(&json!({"limit": u64::MAX}), 50, 200), 200);
    }

    #[test]
    fn parse_limit_allows_values_below_max() {
        assert_eq!(parse_limit(&json!({"limit": 7}), 20, 50), 7);
    }

    #[test]
    fn parse_limit_ignores_non_numeric_values() {
        assert_eq!(parse_limit(&json!({"limit": "10"}), 20, 50), 20);
    }
}
