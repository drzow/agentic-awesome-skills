use std::collections::HashSet;

use crate::models::{CatalogIndex, SearchResult, SkillEntry};
use crate::search::tokenizer;

/// Search the catalog index for skills matching the query.
pub fn search(index: &CatalogIndex, query: &str, limit: usize) -> Vec<SearchResult> {
    let query_tokens = tokenizer::tokenize_query(query);

    if query_tokens.is_empty() {
        return Vec::new();
    }

    let mut scored: Vec<_> = index.skills.iter()
        .filter_map(|skill| {
            let matched: Vec<&str> = query_tokens.iter()
                .filter_map(|t| {
                    if skill.search_tokens.contains(t.as_str()) || skill.id.contains(t.as_str()) {
                        Some(t.as_str())
                    } else {
                        None
                    }
                })
                .collect();

            if matched.is_empty() {
                None
            } else {
                let score = compute_relevance_score(skill, &matched);
                Some(SearchResult {
                    skill_id: skill.id.clone(),
                    name: skill.name.clone(),
                    description: skill.description.clone(),
                    category: skill.category.clone(),
                    tags: skill.tags.clone(),
                    risk: skill.risk.clone(),
                    score,
                    matched_tokens: matched.into_iter().map(|s| s.to_string()).collect(),
                })
            }
        })
        .collect();

    scored.sort_by(|a, b| {
        b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal)
    });

    scored.into_iter().take(limit).collect()
}

/// Filter skills by structured criteria without free-text search.
pub fn filter(
    index: &CatalogIndex,
    category: Option<&str>,
    risk: Option<&str>,
    tags: &[&str],
    limit: usize,
) -> Vec<SearchResult> {
    let skill_tags_set: HashSet<&str> = tags.iter().copied().collect();

    index.skills.iter()
        .filter(|s| {
            if let Some(cat) = category {
                if s.category != cat {
                    return false;
                }
            }
            if let Some(r) = risk {
                if s.risk != r {
                    return false;
                }
            }
            if !tags.is_empty() {
                let stags: HashSet<&str> = s.tags.iter().map(|t| t.as_str()).collect();
                if !skill_tags_set.iter().all(|t| stags.contains(*t)) {
                    return false;
                }
            }
            true
        })
        .map(|s| SearchResult {
            skill_id: s.id.clone(),
            name: s.name.clone(),
            description: s.description.clone(),
            category: s.category.clone(),
            tags: s.tags.clone(),
            risk: s.risk.clone(),
            score: 1.0,
            matched_tokens: Vec::new(),
        })
        .take(limit)
        .collect()
}

/// Compute relevance score for a skill based on matched tokens.
fn compute_relevance_score(skill: &SkillEntry, matched_tokens: &[&str]) -> f32 {
    let mut score = 0.0;
    for token in matched_tokens {
        if skill.id.contains(*token) || skill.name.contains(*token) {
            score += 3.0;
        } else if skill.description.contains(*token) {
            score += 2.0;
        } else if skill.tags.iter().any(|t| t.contains(*token)) {
            score += 1.0;
        } else {
            score += 0.5;
        }
    }

    let bonus = (matched_tokens.len() as f32).min(5.0) * 0.2;
    score + bonus
}

/// Get all categories with counts from the index.
pub fn list_categories(index: &CatalogIndex) -> Vec<(String, usize)> {
    let mut counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for skill in &index.skills {
        *counts.entry(skill.category.clone()).or_insert(0) += 1;
    }

    let mut result: Vec<_> = counts.into_iter().collect();
    result.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_skill(id: &str, name: &str, desc: &str, category: &str, tags: &[&str], risk: &str) -> SkillEntry {
        let mut token_set = HashSet::new();
        let all_text = format!("{} {} {} {} {}", id, name, desc, category, tags.join(" "));
        for t in tokenizer::tokenize(&all_text) {
            token_set.insert(t);
        }

        SkillEntry {
            id: id.to_string(),
            name: name.to_string(),
            description: desc.to_string(),
            category: category.to_string(),
            tags: tags.iter().map(|s| s.to_string()).collect(),
            risk: risk.to_string(),
            search_tokens: token_set,
        }
    }

    #[test]
    fn test_search_basic() {
        let skills = vec![
            make_skill("security-audit", "Security Audit", "Perform security audits and penetration testing", "security", &["audit", "pentest"], "critical"),
            make_skill("brainstorming", "Brainstorming", "Facilitate creative brainstorming sessions", "ideation", &["creative", "ideas"], "safe"),
        ];
        let index = CatalogIndex {
            schema_version: 1,
            catalog_digest: "sha256:test".to_string(),
            generated_at: chrono::Utc::now(),
            skill_count: skills.len(),
            skills,
        };

        let results = search(&index, "security", 10);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].skill_id, "security-audit");
    }

    #[test]
    fn test_filter_by_category() {
        let skills = vec![
            make_skill("sec-1", "Security Tool", "A security tool", "security", &["security"], "critical"),
            make_skill("dev-1", "Dev Tool", "A dev tool", "development", &["dev"], "safe"),
        ];
        let index = CatalogIndex {
            schema_version: 1,
            catalog_digest: "sha256:test".to_string(),
            generated_at: chrono::Utc::now(),
            skill_count: skills.len(),
            skills,
        };

        let results = filter(&index, Some("security"), None, &[], 10);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].skill_id, "sec-1");
    }

    #[test]
    fn test_filter_by_tags() {
        let skills = vec![
            make_skill("s1", "Skill One", "Description", "general", &["security", "audit"], "safe"),
            make_skill("s2", "Skill Two", "Description", "general", &["security"], "safe"),
        ];
        let index = CatalogIndex {
            schema_version: 1,
            catalog_digest: "sha256:test".to_string(),
            generated_at: chrono::Utc::now(),
            skill_count: skills.len(),
            skills,
        };

        let results = filter(&index, None, None, &["security", "audit"], 10);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].skill_id, "s1");
    }

    #[test]
    fn test_list_categories() {
        let skills = vec![
            make_skill("s1", "", "", "security", &[], "safe"),
            make_skill("s2", "", "", "security", &[], "safe"),
            make_skill("s3", "", "", "devops", &[], "safe"),
        ];
        let index = CatalogIndex {
            schema_version: 1,
            catalog_digest: "sha256:test".to_string(),
            generated_at: chrono::Utc::now(),
            skill_count: skills.len(),
            skills,
        };

        let cats = list_categories(&index);
        assert_eq!(cats.len(), 2);
        assert_eq!(cats[0].0, "security");
        assert_eq!(cats[0].1, 2);
    }
}
