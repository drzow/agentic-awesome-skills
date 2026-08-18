use std::path::{Path, PathBuf};

/// Validate that a path is safe (no directory traversal).
#[allow(dead_code)] // Public API; used in tests
pub fn validate_path(requested: &str) -> Result<PathBuf, String> {
    if requested.contains("..") {
        return Err(format!("Path contains traversal attempt: {}", requested));
    }

    if requested.contains('\0') || requested.contains('\n') || requested.contains('\r') {
        return Err("Path contains invalid characters".to_string());
    }

    let path = PathBuf::from(requested);

    if !path.is_absolute() && !requested.starts_with("/") {
        return Err(format!("Path is not absolute: {}", requested));
    }

    Ok(path)
}

/// Check that a skill_id is safe for use as a directory/file name.
pub fn validate_skill_id(skill_id: &str) -> Result<(), String> {
    if skill_id.is_empty() {
        return Err("Skill ID cannot be empty".to_string());
    }

    for ch in skill_id.chars() {
        if !ch.is_alphanumeric() && ch != '-' && ch != '_' && ch != '.' {
            return Err(format!("Skill ID contains invalid character: '{}'", ch));
        }
    }

    if skill_id.contains("..") {
        return Err("Skill ID contains path traversal".to_string());
    }

    Ok(())
}

/// Check if a path is contained within a parent directory.
#[allow(dead_code)] // Public API; used in tests
pub fn is_within(parent: &Path, child: &Path) -> bool {
    child.strip_prefix(parent).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_skill_id_valid() {
        assert!(validate_skill_id("brainstorming").is_ok());
        assert!(validate_skill_id("my-skill-id").is_ok());
        assert!(validate_skill_id("skill_v2").is_ok());
        assert!(validate_skill_id("007").is_ok());
    }

    #[test]
    fn test_validate_skill_id_invalid() {
        assert!(validate_skill_id("").is_err());
        assert!(validate_skill_id("../etc/passwd").is_err());
        assert!(validate_skill_id("skill with spaces").is_err());
        assert!(validate_skill_id("script;rm -rf /").is_err());
    }

    #[test]
    fn test_validate_path_traversal() {
        let result = validate_path("../../etc/passwd");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_path_safe() {
        let result = validate_path("/skills/brainstorming/SKILL.md");
        assert!(result.is_ok());
    }

    #[test]
    fn test_is_within() {
        assert!(is_within(Path::new("/home/user"), Path::new("/home/user/skills")));
        assert!(!is_within(Path::new("/home/user"), Path::new("/home/other/skills")));
    }
}
