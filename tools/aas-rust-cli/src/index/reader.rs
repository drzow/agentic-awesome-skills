use anyhow::{anyhow, Result};
use std::path::Path;

use crate::models::CatalogIndex;

/// Load and validate a CatalogIndex from disk.
pub fn load_index(path: &Path) -> Result<CatalogIndex> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| anyhow!("failed to read index at {:?}: {}", path, e))?;

    let index: CatalogIndex = serde_json::from_str(&content)
        .map_err(|e| anyhow!("failed to parse index: {}", e))?;

    if index.schema_version != 1 {
        return Err(anyhow!("unsupported index schema version: {}", index.schema_version));
    }

    Ok(index)
}
