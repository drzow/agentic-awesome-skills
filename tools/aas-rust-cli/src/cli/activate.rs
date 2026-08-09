use anyhow::{anyhow, Result};
use std::fs;
use std::path::PathBuf;
#[cfg(unix)]
use std::os::unix::fs::symlink;
#[cfg(not(unix))]
use std::fs::symlink;

use crate::store::bare_repo::BareStore;
use crate::utils::path_validation;
use crate::utils::platform;

/// Activate or deactivate skills by creating/removing symlinks (or copies) in agent directories.
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

    for skill_id in skill_ids {
        // Validate skill ID
        if let Err(e) = path_validation::validate_skill_id(skill_id) {
            eprintln!("Warning: invalid skill ID '{}': {}", skill_id, e);
            continue;
        }

        // Verify skill exists in store
        if store.get_blob_at_path(skill_id).is_err() {
            eprintln!("Warning: skill '{}' not found in store, skipping.", skill_id);
            continue;
        }

        for target_dir in &targets {
            let dest = target_dir.join(skill_id);

            match action {
                "activate" => {
                    if !dest.exists() || dest.is_symlink() {
                        // Remove stale entry first
                        if dest.exists() && !dest.is_symlink() {
                            fs::remove_dir_all(&dest).ok();
                        }

                        // Try symlink first, then fall back to copy
                        if supports_symlinks && symlink(
                            &store_path.join("skills").join(skill_id),
                            &dest
                        ).is_err() {
                            // Fallback: copy SKILL.md only
                            fs::create_dir_all(&dest).ok();
                            let content = store.get_blob_at_path(skill_id)?;
                            fs::write(dest.join("SKILL.md"), &content).ok();
                            println!("  Activated '{}' -> {} (copy mode)", skill_id, target_dir.display());
                        } else {
                            println!("  Activated '{}' -> {} (symlink)", skill_id, target_dir.display());
                        }
                    } else {
                        println!("  Already activated: '{}' -> {}", skill_id, dest.display());
                    }
                }
                "deactivate" => {
                    if dest.exists() {
                        fs::remove_dir_all(&dest).ok();
                        println!("  Deactivated: '{}' from {}", skill_id, target_dir.display());
                    } else {
                        println!("  Not active: '{}' in {}", skill_id, target_dir.display());
                    }
                }
                _ => return Err(anyhow!("unknown action: {}", action)),
            }
        }
    }

    Ok(())
}
