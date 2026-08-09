use anyhow::{anyhow, Result};
use std::path::Path;

pub mod bare_repo;

/// Open an existing bare store from a path.
#[allow(dead_code)] // TODO(#2): wire into CLI status or init command
pub fn open_store(store_path: &Path) -> Result<bare_repo::BareStore> {
    bare_repo::BareStore::open(store_path)
        .map_err(|e| anyhow!("failed to open store at {:?}: {}", store_path, e))
}
