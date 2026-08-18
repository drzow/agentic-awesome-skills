use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Lightweight skill entry for index-based discovery.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SkillEntry {
    pub id: String,
    pub name: String,
    pub description: String,
    pub category: String,
    #[serde(default)]
    pub tags: Vec<String>,
    pub risk: String,
    #[serde(rename = "searchTokens", default)]
    pub search_tokens: HashSet<String>,
}

/// The full compact catalog index.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CatalogIndex {
    pub schema_version: u32,
    #[serde(rename = "catalogDigest")]
    pub catalog_digest: String,
    #[serde(rename = "generatedAt")]
    pub generated_at: chrono::DateTime<chrono::Utc>,
    #[serde(rename = "catalogVersion", default)]
    pub catalog_version: Option<String>,
    pub skill_count: usize,
    pub skills: Vec<SkillEntry>,
}

/// Clone state stored in ~/.aas/meta/state.json.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CloneState {
    #[serde(rename = "repoUrl")]
    pub repo_url: String,
    #[serde(rename = "sourceSha")]
    pub source_sha: String,
    #[serde(rename = "clonedAt")]
    pub cloned_at: chrono::DateTime<chrono::Utc>,
    #[serde(rename = "lastUpdated")]
    pub last_updated: chrono::DateTime<chrono::Utc>,
    pub version: String,
}

/// A search result with relevance score.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SearchResult {
    pub skill_id: String,
    pub name: String,
    pub description: String,
    pub category: String,
    pub tags: Vec<String>,
    pub risk: String,
    pub score: f32,
    #[serde(rename = "matchedTokens")]
    pub matched_tokens: Vec<String>,
}

/// Category count for list_categories output.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CategoryCount {
    pub category: String,
    pub count: usize,
}
