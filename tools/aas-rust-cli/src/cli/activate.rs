use anyhow::{anyhow, Result};
use std::fs;
use std::path::{Path, PathBuf};

use crate::store::bare_repo::BareStore;
use crate::utils::path_validation;
use crate::utils::platform;

/// Activate or deactivate skills by materializing the full skill tree from
/// the store and linking (or copying) it into agent directories.
pub fn run(base_dir: &PathBuf, action: &str, skill_ids: &[String], target_names: &[String]) -> Result<()> {
    let store_path = base_dir.join("store");
    if !store_path.exists() {
        return Err(anyhow!("store not found. Run 'aas init' first."));
    }

    let store = BareStore::open(&store_path)?;
    let supports_symlinks = platform::supports_symlinks();

    // Determine target directories
    let targets: Vec<PathBuf> = if target_names.is_empty() {
        // Default: use all known agent dirs
        platform::agent_skill_dirs().into_iter().map(|(_, p)| p).collect()
    } else {
        let mut result = Vec::new();
        for name in target_names {
            if let Some(dir) = platform::get_target(name) {
                result.push(dir);
            } else {
                eprintln!("Warning: unknown target '{}'", name);
            }
        }
        if result.is_empty() {
            return Err(anyhow!("no valid targets specified"));
        }
        result
    };

    // Materialized skills live in base_dir/skills/{id}, next to store/.
    let materialized_root = base_dir.join("skills");
    let mut failures = 0usize;

    for skill_id in skill_ids {
        // Validate skill ID
        if let Err(e) = path_validation::validate_skill_id(skill_id) {
            eprintln!("Warning: invalid skill ID '{}': {}", skill_id, e);
            failures += 1;
            continue;
        }

        match action {
            "activate" => {
                fs::create_dir_all(&materialized_root)
                    .map_err(|e| anyhow!("failed to create {}: {}", materialized_root.display(), e))?;

                let materialized = materialized_root.join(skill_id);
                let temp_root = materialized_root.join(format!(".tmp-{}-{}", skill_id, std::process::id()));
                let _ = remove_entry(&temp_root);

                if let Err(e) = store.extract_skill(skill_id, &temp_root) {
                    eprintln!("Warning: skill '{}' not found in store, skipping. ({})", skill_id, e);
                    let _ = remove_entry(&temp_root);
                    failures += 1;
                    continue;
                }

                let temp_skill = temp_root.join(skill_id);
                if let Err(e) = remove_entry(&materialized) {
                    eprintln!("Warning: could not remove existing materialized skill '{}': {}", skill_id, e);
                    let _ = remove_entry(&temp_root);
                    failures += 1;
                    continue;
                }

                if let Err(e) = fs::rename(&temp_skill, &materialized) {
                    eprintln!("Warning: could not materialize skill '{}': {}", skill_id, e);
                    let _ = remove_entry(&temp_root);
                    failures += 1;
                    continue;
                }
                let _ = remove_entry(&temp_root);

                for target_dir in &targets {
                    let dest = target_dir.join(skill_id);
                    if let Err(e) = install_target(&materialized, &dest, skill_id, supports_symlinks) {
                        eprintln!("Warning: failed to activate '{}' in {}: {}", skill_id, target_dir.display(), e);
                        failures += 1;
                    }
                }
            }
            "deactivate" => {
                for target_dir in &targets {
                    let dest = target_dir.join(skill_id);
                    if dest.exists() || dest.is_symlink() {
                        match remove_entry(&dest) {
                            Ok(()) => println!("  Deactivated: '{}' from {}", skill_id, target_dir.display()),
                            Err(e) => {
                                eprintln!("Warning: failed to deactivate '{}' from {}: {}", skill_id, target_dir.display(), e);
                                failures += 1;
                            }
                        }
                    } else {
                        println!("  Not active: '{}' in {}", skill_id, target_dir.display());
                    }
                }
            }
            _ => return Err(anyhow!("unknown action: {}", action)),
        }
    }

    if failures > 0 {
        Err(anyhow!("{} operation(s) failed", failures))
    } else {
        Ok(())
    }
}

/// Install a materialized skill into one agent target directory.
fn install_target(materialized: &Path, dest: &Path, skill_id: &str, supports_symlinks: bool) -> Result<()> {
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent)?;
    }
    remove_entry(dest)?;

    if supports_symlinks {
        match platform::create_symlink(materialized, dest) {
            Ok(()) => {
                println!("  Activated '{}' -> {} (symlink)", skill_id, dest.display());
                return Ok(());
            }
            Err(e) => {
                eprintln!(
                    "Warning: symlink failed for '{}' to {} ({}); falling back to copy",
                    skill_id,
                    dest.display(),
                    e
                );
                remove_entry(dest)?;
            }
        }
    }

    copy_dir(materialized, dest)?;
    println!("  Activated '{}' -> {} (copy mode)", skill_id, dest.display());
    Ok(())
}

/// Remove an existing entry (symlink, directory, or file) at `path`.
/// No-op if the path does not exist.
fn remove_entry(path: &Path) -> Result<()> {
    if path.is_symlink() {
        fs::remove_file(path)?;
    } else if path.is_dir() {
        fs::remove_dir_all(path)?;
    } else if path.exists() {
        fs::remove_file(path)?;
    }
    Ok(())
}

/// Recursively copy a directory tree from `src` to `dest`.
fn copy_dir(src: &Path, dest: &Path) -> Result<()> {
    fs::create_dir_all(dest)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let target = dest.join(entry.file_name());
        remove_entry(&target)?;

        if file_type.is_dir() {
            copy_dir(&entry.path(), &target)?;
        } else if file_type.is_symlink() {
            let link_target = fs::read_link(entry.path())?;
            platform::create_symlink(&link_target, &target)
                .map_err(|e| anyhow!("failed to symlink {} -> {}: {}", link_target.display(), target.display(), e))?;
        } else {
            fs::copy(entry.path(), &target)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_remove_entry_noop_when_missing() {
        let tmp = tempfile::tempdir().unwrap();
        remove_entry(&tmp.path().join("missing")).unwrap();
    }

    #[test]
    fn test_remove_entry_file() {
        let tmp = tempfile::tempdir().unwrap();
        let file = tmp.path().join("file.txt");
        fs::write(&file, "data").unwrap();
        remove_entry(&file).unwrap();
        assert!(!file.exists());
    }

    #[test]
    fn test_remove_entry_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("dir");
        fs::create_dir_all(dir.join("nested")).unwrap();
        fs::write(dir.join("nested").join("file.txt"), "data").unwrap();
        remove_entry(&dir).unwrap();
        assert!(!dir.exists());
    }

    #[test]
    fn test_copy_dir_nested() {
        let tmp = tempfile::tempdir().unwrap();
        let src = tmp.path().join("src");
        fs::create_dir_all(src.join("references")).unwrap();
        fs::write(src.join("SKILL.md"), "skill").unwrap();
        fs::write(src.join("references").join("x.md"), "ref").unwrap();

        let dest = tmp.path().join("dest");
        copy_dir(&src, &dest).unwrap();

        assert_eq!(fs::read_to_string(dest.join("SKILL.md")).unwrap(), "skill");
        assert_eq!(fs::read_to_string(dest.join("references").join("x.md")).unwrap(), "ref");
    }

    #[cfg(unix)]
    #[test]
    fn test_copy_dir_preserves_symlinks() {
        let tmp = tempfile::tempdir().unwrap();
        let src = tmp.path().join("src");
        fs::create_dir_all(&src).unwrap();
        fs::write(src.join("real.txt"), "data").unwrap();
        std::os::unix::fs::symlink(src.join("real.txt"), src.join("link.txt")).unwrap();

        let dest = tmp.path().join("dest");
        copy_dir(&src, &dest).unwrap();

        assert!(dest.join("link.txt").is_symlink());
        assert_eq!(fs::read_to_string(dest.join("link.txt")).unwrap(), "data");
    }
}