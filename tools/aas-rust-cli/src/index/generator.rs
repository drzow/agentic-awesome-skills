use anyhow::Result;
use chrono::Utc;
use std::collections::HashSet;

use crate::models::{CatalogIndex, SkillEntry};
use crate::search::tokenizer;
use crate::store::bare_repo::BareStore;

/// Generate a compact CatalogIndex from the git store.
pub fn generate_index(store: &BareStore) -> Result<CatalogIndex> {
    let skill_dirs = store.list_skill_dirs()?;
    let mut skills = Vec::with_capacity(skill_dirs.len());

    for skill_id in &skill_dirs {
        if let Ok(content) = store.get_blob_at_path(skill_id) {
            if let Ok(text) = String::from_utf8(content.clone()) {
                if let Some(entry) = parse_skill_entry(skill_id, &text) {
                    skills.push(entry);
                }
            }
        }
    }

    let catalog_digest = store.tree_sha().unwrap_or_else(|_| "unknown".to_string());
    let catalog_version = store.catalog_version().ok().flatten();

    Ok(CatalogIndex {
        schema_version: 1,
        catalog_digest,
        generated_at: Utc::now(),
        catalog_version,
        skill_count: skills.len(),
        skills,
    })
}

/// Parse YAML frontmatter from a SKILL.md file into a SkillEntry.
fn parse_skill_entry(id: &str, content: &str) -> Option<SkillEntry> {
    let fm = extract_frontmatter(content)?;

    let get_field = |key: &str| -> String {
        fm.get(key).cloned().unwrap_or_default()
    };

    let name = get_field("name");
    let description = get_field("description");
    let category = get_field("category");
    let risk = get_field("risk");

    // Parse tags
    let tags: Vec<String> = fm.get("tags")
        .map(|t| {
            t.split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect()
        })
        .unwrap_or_default();

    // Build search tokens from all fields
    let mut token_set = HashSet::new();
    let all_text = format!("{} {} {} {} {}", id, name, description, category, tags.join(" "));
    for t in tokenizer::tokenize(&all_text) {
        token_set.insert(t);
    }

    Some(SkillEntry {
        id: id.to_string(),
        name,
        description,
        category,
        tags,
        risk,
        search_tokens: token_set,
    })
}

/// Extract YAML frontmatter from content between ---delimiters.
fn extract_frontmatter(content: &str) -> Option<std::collections::HashMap<String, String>> {
    let content = content.trim_start();
    if !content.starts_with("---") {
        return None;
    }

    let rest = &content[3..];
    let end_pos = rest.find("---").or_else(|| rest.find("=== "))?;
    let fm_text = &rest[..end_pos].trim();

    let mut map = std::collections::HashMap::new();
    for line in fm_text.lines() {
        let line = line.trim();
        if let Some((key, value)) = line.split_once(':') {
            let key = key.trim().to_string();
            let value = value.trim().trim_matches('"').trim_matches('\'').to_string();
            map.insert(key, value);
        }
    }

    if map.is_empty() { None } else { Some(map) }
}
