use anyhow::{anyhow, Result};
use git2::{CertificateCheckStatus, FetchOptions, RemoteCallbacks, Repository, Cred};
use std::path::Path;

/// A bare-git-backed skill store.
///
/// The store is a bare repository that may be fetched from over HTTPS or SSH.
/// TLS verification can be disabled via `AAS_SKIP_TLS_VERIFY` for internal
/// registries with self-signed certificates.
pub struct BareStore {
    pub store_path: String,
}

impl BareStore {
    /// Open an existing bare store at the given path.
    pub fn open(store_path: &Path) -> Result<Self> {
        if !store_path.exists() {
            return Err(anyhow!("store path does not exist: {:?}", store_path));
        }
        Ok(Self {
            store_path: store_path.to_string_lossy().to_string(),
        })
    }

    /// Initialise a new bare store by cloning from the given URL.
    pub fn init(repo_url: &str, store_path: &Path) -> Result<Self> {
        println!("Cloning {} into {:?}", repo_url, store_path);

        // Configure callbacks for TLS / SSH.
        let mut remote_callbacks = RemoteCallbacks::new();

        if should_skip_tls() {
            eprintln!(
                "WARNING: AAS_SKIP_TLS_VERIFY is set — all certificate verification is DISABLED (both HTTPS TLS and SSH hostkey)."
            );
        }

        remote_callbacks.certificate_check(|_cert, url| {
            if should_skip_tls() {
                return Ok(CertificateCheckStatus::CertificateOk);
            }
            println!("Certificate check for {}: skipping (use native TLS in production)", url);
            Ok(CertificateCheckStatus::CertificatePassthrough)
        });

        // SSH credential callback with explicit fallback chain.
        // Attempt order:
        // 1. `ssh-agent` — uses any keys loaded in the running agent.
        // 2. Filesystem keys — tries ~/.ssh/id_rsa, id_ed25519, id_ecdsa.
        // 3. Default (Cred::default()) — falls back to git2's default
        //    credential resolution, which may ask the user interactively or
        //    delegate to an external helper (e.g. OS keychain, gpg-agent).
        remote_callbacks.credentials(|_url, username_from_url, _allowed| {
            let user = username_from_url.unwrap_or("git");
            if let Ok(builder) = Cred::ssh_key_from_agent(user) {
                return Ok(builder);
            }
            let home = dirs::home_dir().map(|h| h.to_path_buf()).unwrap_or_default();
            for key_path in [
                home.join(".ssh").join("id_rsa"),
                home.join(".ssh").join("id_ed25519"),
                home.join(".ssh").join("id_ecdsa"),
            ] {
                if key_path.exists() {
                    if let Ok(builder) = Cred::ssh_key_from_memory(
                        user,
                        None,
                        &std::fs::read_to_string(&key_path).ok().unwrap_or_default(),
                        None,
                    ) {
                        return Ok(builder);
                    }
                }
            }
            // Cred::default() does not panic — it returns a credential that
            // will ask the user interactively or delegate to an external helper.
            Cred::default()
        });

        let mut fetch_opts = FetchOptions::new();
        fetch_opts.remote_callbacks(remote_callbacks);

        // Perform initial clone into the store path.
        Repository::init_bare(store_path)?;
        let repo = Repository::open(store_path)?;
        let mut origin = repo.remote_anonymous(repo_url)?;
        origin.fetch(&mut ["+refs/heads/*:refs/heads/*"], Some(&mut fetch_opts), None)
            .map_err(|e| anyhow!("clone failed: {}", e))?;

        // After fetching a bare repo, HEAD may not point to any existing branch.
        // Discover the default branch and set HEAD as a symbolic ref to it.
        let default_branch = Self::default_branch_name(&repo)?;
        if let Ok(mut head_ref) = repo.find_reference("HEAD") {
            if head_ref.symbolic_target().is_some() {
                head_ref.symbolic_set_target(
                    &format!("refs/heads/{}", default_branch),
                    "init: set HEAD to default branch",
                )?;
            } else {
                repo.reference_symbolic("HEAD", &format!("refs/heads/{}", default_branch), true, "init: set HEAD to ...")?;
            }
        } else {
            repo.reference_symbolic("HEAD", &format!("refs/heads/{}", default_branch), true, "init: set HEAD to ...")?;
        }

        Ok(Self {
            store_path: store_path.to_string_lossy().to_string(),
        })
    }

    /// Fetch latest from origin, returning the new HEAD SHA.
    pub fn fetch(&self) -> Result<String> {
        let repo = Repository::open_bare(&self.store_path)?;

        let mut callbacks = RemoteCallbacks::new();

        if should_skip_tls() {
            eprintln!(
                "WARNING: AAS_SKIP_TLS_VERIFY is set — all certificate verification is DISABLED (both HTTPS TLS and SSH hostkey)."
            );
        }

        callbacks.certificate_check(|_cert, url| {
            if should_skip_tls() {
                return Ok(CertificateCheckStatus::CertificateOk);
            }
            println!("Certificate check for {}: skipping (use native TLS in production)", url);
            Ok(CertificateCheckStatus::CertificatePassthrough)
        });

        // SSH credential callback with explicit fallback chain.
        // Attempt order:
        // 1. `ssh-agent` — uses any keys loaded in the running agent.
        // 2. Filesystem keys — tries ~/.ssh/id_rsa, id_ed25519, id_ecdsa.
        // 3. Default (Cred::default()) — falls back to git2's default
        //    credential resolution, which may ask the user interactively or
        //    delegate to an external helper (e.g. OS keychain, gpg-agent).
        callbacks.credentials(|_url, username_from_url, _allowed| {
            let user = username_from_url.unwrap_or("git");
            if let Ok(builder) = Cred::ssh_key_from_agent(user) {
                return Ok(builder);
            }
            let home = dirs::home_dir().map(|h| h.to_path_buf()).unwrap_or_default();
            for key_path in [
                home.join(".ssh").join("id_rsa"),
                home.join(".ssh").join("id_ed25519"),
                home.join(".ssh").join("id_ecdsa"),
            ] {
                if key_path.exists() {
                    if let Ok(builder) = Cred::ssh_key_from_memory(
                        user,
                        None,
                        &std::fs::read_to_string(&key_path).ok().unwrap_or_default(),
                        None,
                    ) {
                        return Ok(builder);
                    }
                }
            }
            // Cred::default() does not panic — it returns a credential that
            // will ask the user interactively or delegate to an external helper.
            Cred::default()
        });

        let mut fetch_opts = FetchOptions::new();
        fetch_opts.remote_callbacks(callbacks);

        let mut origin = repo.find_remote("origin")?;
        origin.fetch(&mut ["+refs/heads/*:refs/heads/*"], Some(&mut fetch_opts), None)
            .map_err(|e| anyhow!("fetch failed: {}", e))?;

        // Return the new HEAD SHA.
        let head = repo.head()?;
        Ok(head.target().ok_or_else(|| anyhow!("no HEAD after fetch"))?.to_string())
    }

    /// Discover the primary branch name for a bare repository using a 3-tier strategy:
    /// 1. Read `init.defaultBranch` from repo config
    /// 2. Resolve symbolic HEAD (e.g., `refs/heads/main`) in the bare repo
    /// 3. Scan `refs/heads/*`, preferring common names in order: main, master, default, trunk
    fn default_branch_name(repo: &Repository) -> Result<String> {
        // Strategy 1: check git config init.defaultBranch
        if let Ok(cfg) = repo.config() {
            if let Ok(val) = cfg.get_string("init.defaultBranch") {
                let full_ref = format!("refs/heads/{}", val);
                if repo.refname_to_id(&full_ref).is_ok() {
                    return Ok(val);
                }
            }
        }

        // Strategy 2: resolve HEAD to get the default branch
        if let Ok(head) = repo.head() {
            if let Some(refname) = head.symbolic_target() {
                if refname.starts_with("refs/heads/") {
                    if let Ok(oid) = repo.refname_to_id(refname) {
                        if repo.find_object(oid, None).map(|o| o.kind() == Some(git2::ObjectType::Commit)).unwrap_or(false) {
                            if let Some(branch) = refname.strip_prefix("refs/heads/") {
                                return Ok(branch.to_string());
                            }
                        }
                    }
                }
            }
        }

        // Strategy 3: iterate refs and check for common branch names in order of preference
        let candidates = ["main", "master", "default", "trunk"];
        let mut best_match: Option<(usize, String)> = None;
        let mut found_any_ref = false;

        for entry in repo.references()? {
            let entry = entry?;
            if let Some(name) = entry.name() {
                if let Some(branch) = name.strip_prefix("refs/heads/") {
                    found_any_ref = true;
                    for (idx, candidate) in candidates.iter().enumerate() {
                        if branch == *candidate {
                            match &best_match {
                                Some((best_idx, _)) if *best_idx <= idx => {}
                                _ => { best_match = Some((idx, candidate.to_string())); }
                            }
                            break;
                        }
                    }
                }
            }
        }

        if let Some((_idx, branch)) = best_match {
            return Ok(branch);
        }

        // If no common name matched but branches exist, return the first one found
        if found_any_ref {
            for entry in repo.references()? {
                let entry = entry?;
                if let Some(name) = entry.name() {
                    if let Some(branch) = name.strip_prefix("refs/heads/") {
                        return Ok(branch.to_string());
                    }
                    // Also check remote tracking refs as fallback
                    if let Some(branch) = name.strip_prefix("refs/remotes/origin/") {
                        return Ok(branch.to_string());
                    }
                }
            }
        }

        Err(anyhow!(
            "could not determine default branch: no refs/heads/* found; \
             ensure the repository has at least one branch and fetch completed successfully"
        ))
    }

    /// Update a ref to point at the given SHA, triggering index rebuild.
    pub fn update_ref(&self, sha: &str) -> Result<()> {
        let repo = Repository::open_bare(&self.store_path)?;
        let oid = git2::Oid::from_str(sha)
            .map_err(|e| anyhow!("invalid SHA '{}': {}", sha, e))?;

        let branch_name = Self::default_branch_name(&repo)?;
        let full_ref = if branch_name.starts_with("refs/") {
            branch_name.clone()
        } else {
            format!("refs/heads/{}", branch_name)
        };
        let _current_id = repo.refname_to_id(&full_ref)
            .map_err(|e| anyhow!("{} not found: {}", full_ref, e))?;
        let mut reference = repo.find_reference(&full_ref)?;
        reference.set_target(oid, "update: fetch from origin")?;
        Ok(())
    }

    /// Read the content of a blob at a given path within the bare repository.
    ///
    /// The path is resolved relative to the work tree by walking the tree
    /// hierarchy starting from the HEAD commit.
    pub fn read_blob_at_path(&self, path: &str) -> Result<Vec<u8>> {
        let repo = Repository::open_bare(&self.store_path)?;
        if path.is_empty() {
            return Err(anyhow!("empty path"));
        }
        let head = repo.head()?;
        let commit = repo.find_commit(head.target().unwrap())?;
        let tree = commit.tree()?;

        // Walk the path components.
        let components: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
        let mut current_tree = tree;

        for (i, component) in components.iter().enumerate() {
            let entry_id = current_tree.get_name(*component)
                .map(|e| e.id())
                .ok_or_else(|| anyhow!("path component not found: {}", component))?;

            if i == components.len() - 1 {
                // Last component — read the blob.
                let obj = repo.find_object(entry_id, None)?;
                return obj.as_blob()
                    .map(|b| b.content().to_vec())
                    .ok_or_else(|| anyhow!("not a blob: {}", path));
            }

            // Intermediate component must be a tree.
            let obj = repo.find_object(entry_id, None)?;
            current_tree = obj.as_tree()
                .ok_or_else(|| anyhow!("expected tree, got blob at: {}", component))?
                .to_owned();
        }

        // This branch is reached only when all components were filtered out
        // (e.g. path was "///" or "/"). Treat as empty/invalid path.
        if components.is_empty() {
            return Err(anyhow!("path contains no valid components"));
        }
        unreachable!("loop always returns for non-empty components")
    }

    /// Alias for [`Self::read_blob_at_path`] — returns the raw blob bytes.
    pub fn get_blob_at_path(&self, id: &str) -> Result<Vec<u8>> {
        self.read_blob_at_path(id)
    }

    /// List all skill directory names (subdirectories containing SKILL.md).
    pub fn list_skill_dirs(&self) -> Result<Vec<String>> {
        let repo = Repository::open_bare(&self.store_path)?;
        let head = repo.head()?;
        let commit = repo.find_commit(head.target().unwrap())?;
        let tree = commit.tree()?;

        // List directories at root that contain SKILL.md.
        let mut ids = Vec::new();
        for entry in tree.iter() {
            if let Some(dir_name) = entry.name() {
                let child_obj = entry.to_object(&repo)?;
                if let Some(child_tree) = child_obj.as_tree() {
                    for child_entry in child_tree.iter() {
                        if let Some(child_name) = child_entry.name() {
                            if child_name == "SKILL.md" {
                                ids.push(dir_name.to_string());
                                break;
                            }
                        }
                    }
                }
            }
        }

        ids.sort();
        Ok(ids)
    }

    /// Get the HEAD commit SHA as a hex string.
    pub fn head_sha(&self) -> Result<String> {
        let repo = Repository::open_bare(&self.store_path)?;
        let head = repo.head()?;
        Ok(head.target()
            .map(|oid| oid.to_string())
            .ok_or_else(|| anyhow!("no HEAD"))?)
    }

    /// Get the root tree SHA for catalog digest.
    pub fn tree_sha(&self) -> Result<String> {
        let repo = Repository::open_bare(&self.store_path)?;
        let head = repo.head()?;
        let commit = repo.find_commit(head.target().unwrap())?;
        let tree = commit.tree()?;
        Ok(tree.id().to_string())
    }

    /// Get the number of refs (branches) in the store.
    #[allow(dead_code)] // TODO: wire into CLI status command
    pub fn ref_count(&self) -> Result<usize> {
        let repo = Repository::open_bare(&self.store_path)?;
        let mut count = 0;
        for entry in repo.references()? {
            let _ = entry?;
            count += 1;
        }
        Ok(count)
    }
}

/// Check whether TLS certificate verification should be skipped.
///
/// Returns `true` if the `AAS_SKIP_TLS_VERIFY` environment variable is set to
/// a truthy value (case-insensitive): `1`, `true`, `yes`, or `on`.
pub fn should_skip_tls() -> bool {
    std::env::var("AAS_SKIP_TLS_VERIFY")
        .ok()
        .map(|v| matches!(v.to_lowercase().as_str(), "1" | "true" | "yes" | "on"))
        .unwrap_or(false)
}

/// Open a bare repository from the given path.
#[allow(dead_code)] // TODO: wire into CLI for custom repo opening
pub trait RepositoryExt {
    fn open_bare(path: &Path) -> Result<Repository>;
}

impl RepositoryExt for Repository {
    /// Open an existing bare repository, or return a descriptive error.
    fn open_bare(path: &Path) -> Result<Repository> {
        let repo = Repository::open(path)
            .map_err(|e| anyhow!("failed to open bare repo at {:?}: {}", path, e))?;

        if !repo.is_bare() {
            return Err(anyhow!("repository at {:?} is not bare", path));
        }

        Ok(repo)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_should_skip_tls_defaults_false() {
        // Ensure the env var is unset.
        let _guard = TestEnvGuard::new("AAS_SKIP_TLS_VERIFY", None);
        assert!(!should_skip_tls());
    }

    #[test]
    fn test_should_skip_tls_various_truthy_values() {
        for val in &["1", "true", "yes", "on"] {
            let _guard = TestEnvGuard::new("AAS_SKIP_TLS_VERIFY", Some(val));
            assert!(should_skip_tls(), "expected '{}' to be truthy", val);
        }
    }

    #[test]
    fn test_should_skip_tls_various_falsy_values() {
        for val in &["0", "false", "no", "off", "random"] {
            let _guard = TestEnvGuard::new("AAS_SKIP_TLS_VERIFY", Some(val));
            assert!(!should_skip_tls(), "expected '{}' to be falsy", val);
        }
    }

    #[test]
    fn test_should_skip_tls_case_insensitive() {
        for val in &["TRUE", "True", "YES", "Yes", "ON", "On"] {
            let _guard = TestEnvGuard::new("AAS_SKIP_TLS_VERIFY", Some(val));
            assert!(should_skip_tls(), "expected '{}' (case-insensitive) to be truthy", val);
        }
    }

    struct TestEnvGuard {
        key: String,
        prev: Option<String>,
    }

    impl TestEnvGuard {
        fn new(key: &str, value: Option<&str>) -> Self {
            let prev = env::var(key).ok();
            match value {
                Some(v) => env::set_var(key, v),
                None => { env::remove_var(key); }
            }
            Self { key: key.to_string(), prev }
        }
    }

    impl Drop for TestEnvGuard {
        fn drop(&mut self) {
            match &self.prev {
                Some(v) => env::set_var(&self.key, v),
                None => env::remove_var(&self.key),
            }
        }
    }
}
