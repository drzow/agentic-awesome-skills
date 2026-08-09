use anyhow::{anyhow, Result};
use std::path::PathBuf;

use crate::store::bare_repo::BareStore;
use crate::utils::path_validation;

/// Fetch and print the full SKILL.md for a skill.
pub fn run(base_dir: &PathBuf, skill_id: &str) -> Result<()> {
    // Validate skill ID for safety
    path_validation::validate_skill_id(skill_id)
        .map_err(|e| anyhow!("invalid skill ID: {}", e))?;

    let store_path = base_dir.join("store");
    if !store_path.exists() {
        return Err(anyhow!("store not found. Run 'aas init' first."));
    }

    let store = BareStore::open(&store_path)?;
    let content = store.get_blob_at_path(skill_id)?;
    let text = String::from_utf8_lossy(&content);

    print!("{}", text);
    Ok(())
}
