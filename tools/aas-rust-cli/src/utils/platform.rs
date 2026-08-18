use std::io;
use std::path::{Path, PathBuf};

/// Get common agent skill directories.
pub fn agent_skill_dirs() -> Vec<(String, PathBuf)> {
    let mut dirs = Vec::new();

    if let Some(home) = dirs::home_dir() {
        dirs.push(("opencode".to_string(), home.join(".agents").join("skills")));
        dirs.push(("claude-code".to_string(), home.join(".claude").join("skills")));
        dirs.push(("cursor".to_string(), home.join(".cursor").join("skills")));
        dirs::home_dir().map(|h| dirs.push(("gemini-cli".to_string(), h.join(".gemini").join("skills"))));
        dirs::home_dir().map(|h| dirs.push(("codex".to_string(), h.join(".codex").join("skills"))));
        dirs::home_dir().map(|h| dirs.push(("kiro".to_string(), h.join(".kiro").join("skills"))));
    }

    dirs
}

/// Get the target directory for a named agent.
pub fn get_target(name: &str) -> Option<PathBuf> {
    let home = dirs::home_dir()?;
    match name.to_lowercase().as_str() {
        "opencode" | "antigravity" => Some(home.join(".agents").join("skills")),
        "claude-code" | "claude" => Some(home.join(".claude").join("skills")),
        "cursor" => Some(home.join(".cursor").join("skills")),
        "gemini-cli" | "gemini" => Some(home.join(".gemini").join("skills")),
        "codex" => Some(home.join(".codex").join("skills")),
        "kiro" => Some(home.join(".kiro").join("skills")),
        _ => None,
    }
}

/// Detect if symlinks are supported on this platform.
pub fn supports_symlinks() -> bool {
    #[cfg(any(unix, windows))]
    {
        // Windows may require admin or developer mode; callers fall back to copy.
        true
    }
    #[cfg(not(any(unix, windows)))]
    {
        false
    }
}

/// Create a filesystem symlink from `link` to `target`.
pub fn create_symlink(target: &Path, link: &Path) -> io::Result<()> {
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(target, link)
    }
    #[cfg(windows)]
    {
        if target.is_dir() {
            std::os::windows::fs::symlink_dir(target, link)
        } else {
            std::os::windows::fs::symlink_file(target, link)
        }
    }
    #[cfg(not(any(unix, windows)))]
    {
        let _ = (target, link);
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "symlinks are not supported on this platform",
        ))
    }
}
