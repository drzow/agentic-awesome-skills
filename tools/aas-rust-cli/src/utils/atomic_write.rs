use std::fs;
use std::io::{self, Write};
use std::path::Path;
use tempfile::NamedTempFile;

/// Write data atomically: write to a temp file, then persist (rename) over the destination.
pub fn atomic_write(path: &Path, data: &[u8]) -> Result<(), io::Error> {
    let parent = path.parent().ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "No parent dir"))?;
    fs::create_dir_all(parent)?;
    let tmp = NamedTempFile::new_in(parent)?;
    tmp.as_file().write_all(data)?;
    tmp.persist(path).map_err(|e| io::Error::new(io::ErrorKind::Other, format!("persist: {}", e)))?;
    Ok(())
}
