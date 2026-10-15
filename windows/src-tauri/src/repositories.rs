//! Read-only resolution of a user-selected local Git checkout.
//!
//! The path is always supplied explicitly by the caller. In particular, this
//! module never trusts a hook's working directory when deciding GitHub scope.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use serde::Serialize;

const COMMAND_TIMEOUT: Duration = Duration::from_secs(4);
const MAX_OUTPUT: usize = 64 * 1024;

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RepositoryIdentity {
    pub canonical_path: String,
    pub common_dir: String,
    pub worktree_dir: String,
    pub branch: String,
    pub head_sha: String,
    pub remote_name: String,
    /// A credential-free normalized GitHub URL.
    pub remote_url: String,
    pub owner: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repository_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub worktree_id: Option<String>,
    #[serde(skip)]
    pub(crate) github_remote_repositories: Vec<(String, String)>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RepositoryError {
    InvalidPath,
    NotRepository,
    DetachedHead,
    MissingHead,
    NoGitHubRemote,
    AmbiguousRemote,
    GitUnavailable,
    GitFailure,
}

impl std::fmt::Display for RepositoryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::InvalidPath => "Choose an existing local folder.",
            Self::NotRepository => "The selected folder is not inside a Git repository.",
            Self::DetachedHead => "Check out a branch before connecting GitHub.",
            Self::MissingHead => "The selected repository has no commit yet.",
            Self::NoGitHubRemote => "No supported GitHub remote was found for this branch.",
            Self::AmbiguousRemote => {
                "Several GitHub remotes match. Set the branch's remote or remove the ambiguity."
            }
            Self::GitUnavailable => "Git is unavailable. Install Git for Windows and try again.",
            Self::GitFailure => "Git could not inspect the selected repository.",
        })
    }
}

impl std::error::Error for RepositoryError {}

#[derive(Debug)]
pub(crate) struct CommandOutput {
    pub stdout: Vec<u8>,
    pub success: bool,
    pub timed_out: bool,
}

/// Run one executable without a shell, with bounded output and a hard timeout.
pub(crate) fn run_bounded(
    program: &str,
    args: &[&str],
    timeout: Duration,
) -> Result<CommandOutput, ()> {
    let mut command = Command::new(program);
    command
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        // Keep Git's discovery anchored at the explicit `-C` selection even
        // when the app was launched from a Git-aware shell.
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_COMMON_DIR")
        .env_remove("GIT_INDEX_FILE")
        .env_remove("GIT_PREFIX")
        .env_remove("GIT_OBJECT_DIRECTORY")
        .env_remove("GIT_ALTERNATE_OBJECT_DIRECTORIES")
        .env_remove("GIT_NAMESPACE");
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000);
    }
    let mut child = command.spawn().map_err(|_| ())?;
    let stdout = child.stdout.take().ok_or(())?;
    let stderr = child.stderr.take().ok_or(())?;
    let stdout_reader = std::thread::spawn(move || read_capped(stdout));
    let stderr_reader = std::thread::spawn(move || read_capped(stderr));
    let deadline = Instant::now() + timeout;
    let mut timed_out = false;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break Some(status),
            Ok(None) if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(20)),
            Ok(None) => {
                timed_out = true;
                let _ = child.kill();
                break child.wait().ok();
            }
            Err(_) => {
                let _ = child.kill();
                let _ = child.wait();
                break None;
            }
        }
    };
    let stdout = stdout_reader.join().unwrap_or_default();
    let _ = stderr_reader.join();
    Ok(CommandOutput {
        stdout,
        success: status.is_some_and(|status| status.success()),
        timed_out,
    })
}

fn read_capped(mut reader: impl std::io::Read) -> Vec<u8> {
    let mut result = Vec::new();
    let mut buffer = [0u8; 4096];
    while result.len() < MAX_OUTPUT {
        let remaining = (MAX_OUTPUT - result.len()).min(buffer.len());
        match reader.read(&mut buffer[..remaining]) {
            Ok(0) | Err(_) => break,
            Ok(count) => result.extend_from_slice(&buffer[..count]),
        }
    }
    result
}

fn git(path: &Path, args: &[&str]) -> Result<String, RepositoryError> {
    let mut command_args = vec!["-C", path.to_str().ok_or(RepositoryError::InvalidPath)?];
    command_args.extend_from_slice(args);
    let output = run_bounded("git", &command_args, COMMAND_TIMEOUT)
        .map_err(|_| RepositoryError::GitUnavailable)?;
    if output.timed_out || !output.success {
        return Err(RepositoryError::GitFailure);
    }
    String::from_utf8(output.stdout)
        .map(|value| value.trim().to_string())
        .map_err(|_| RepositoryError::GitFailure)
}

/// Resolve canonical repository and worktree identity from an explicit path.
pub fn resolve_repository(path: &str) -> Result<RepositoryIdentity, RepositoryError> {
    let input = Path::new(path);
    if !input.is_absolute() || !input.is_dir() {
        return Err(RepositoryError::InvalidPath);
    }
    let canonical = input
        .canonicalize()
        .map_err(|_| RepositoryError::InvalidPath)?;
    let top = git(&canonical, &["rev-parse", "--show-toplevel"]).map_err(map_repo_error)?;
    let top = PathBuf::from(top)
        .canonicalize()
        .map_err(|_| RepositoryError::GitFailure)?;
    let common_dir = git(
        &canonical,
        &["rev-parse", "--path-format=absolute", "--git-common-dir"],
    )
    .map_err(map_repo_error)?;
    let worktree_dir = git(
        &canonical,
        &["rev-parse", "--path-format=absolute", "--git-dir"],
    )
    .map_err(map_repo_error)?;
    let branch = git(&canonical, &["symbolic-ref", "--quiet", "--short", "HEAD"])
        .map_err(|_| RepositoryError::DetachedHead)?;
    let head_sha = git(&canonical, &["rev-parse", "--verify", "HEAD"])
        .map_err(|_| RepositoryError::MissingHead)?;
    if head_sha.len() < 40 || !head_sha.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(RepositoryError::MissingHead);
    }

    let remotes = git(&canonical, &["remote"]).map_err(map_repo_error)?;
    let mut candidates = Vec::new();
    for remote_name in remotes.lines().filter(|name| !name.is_empty()) {
        let urls = git(&canonical, &["remote", "get-url", "--all", remote_name])
            .map_err(map_repo_error)?;
        let mut parsed = Vec::new();
        for value in urls.lines().filter_map(parse_github_remote) {
            if !parsed.contains(&value) {
                parsed.push(value);
            }
        }
        if parsed.len() > 1 {
            return Err(RepositoryError::AmbiguousRemote);
        }
        if let Some((owner, name)) = parsed.into_iter().next() {
            candidates.push((remote_name.to_string(), owner, name));
        }
    }
    if candidates.is_empty() {
        return Err(RepositoryError::NoGitHubRemote);
    }
    let tracking_remote = git(
        &canonical,
        &["config", "--get", &format!("branch.{branch}.remote")],
    )
    .ok();
    let selected = if let Some(remote) = tracking_remote {
        candidates.iter().find(|(name, _, _)| *name == remote)
    } else {
        let origin = candidates.iter().find(|(name, _, _)| name == "origin");
        if origin.is_some() {
            origin
        } else if candidates.len() == 1 {
            candidates.first()
        } else {
            None
        }
    }
    .ok_or(RepositoryError::AmbiguousRemote)?;
    let remote_url = format!("https://github.com/{}/{}", selected.1, selected.2);

    Ok(RepositoryIdentity {
        canonical_path: top.to_string_lossy().into_owned(),
        common_dir: common_dir.trim().to_string(),
        worktree_dir: worktree_dir.trim().to_string(),
        branch,
        head_sha: head_sha.to_ascii_lowercase(),
        remote_name: selected.0.clone(),
        remote_url,
        owner: selected.1.clone(),
        name: selected.2.clone(),
        repository_id: None,
        worktree_id: None,
        github_remote_repositories: candidates
            .iter()
            .map(|(_, owner, name)| (owner.clone(), name.clone()))
            .collect(),
    })
}

fn map_repo_error(error: RepositoryError) -> RepositoryError {
    match error {
        RepositoryError::GitUnavailable | RepositoryError::InvalidPath => error,
        _ => RepositoryError::NotRepository,
    }
}

/// Parse only canonical github.com HTTPS and SSH remotes. Userinfo, custom
/// ports, query strings, fragments, escapes, and extra path components fail.
pub fn parse_github_remote(remote: &str) -> Option<(String, String)> {
    let remote = remote.trim();
    let path = if let Some(value) = strip_ascii_prefix(remote, "https://") {
        let (host, path) = value.split_once('/')?;
        if !host.eq_ignore_ascii_case("github.com") {
            return None;
        }
        path
    } else if let Some(value) = strip_ascii_prefix(remote, "ssh://") {
        let (authority, path) = value.split_once('/')?;
        let (user, host) = authority.split_once('@')?;
        if !user.eq_ignore_ascii_case("git") || !host.eq_ignore_ascii_case("github.com") {
            return None;
        }
        path
    } else if let Some((authority, path)) = remote.split_once(':') {
        let (user, host) = authority.split_once('@')?;
        if !user.eq_ignore_ascii_case("git") || !host.eq_ignore_ascii_case("github.com") {
            return None;
        }
        path
    } else {
        return None;
    };
    if path
        .chars()
        .any(|character| matches!(character, '?' | '#' | '%' | '@' | ':' | '\\'))
    {
        return None;
    }
    let path = path.strip_suffix(".git").unwrap_or(path);
    let mut segments = path.split('/');
    let owner = segments.next()?;
    let name = segments.next()?;
    if segments.next().is_some() || !valid_segment(owner) || !valid_segment(name) {
        return None;
    }
    Some((owner.to_string(), name.to_string()))
}

fn strip_ascii_prefix<'a>(value: &'a str, prefix: &str) -> Option<&'a str> {
    let candidate = value.get(..prefix.len())?;
    candidate
        .eq_ignore_ascii_case(prefix)
        .then(|| &value[prefix.len()..])
}

fn valid_segment(value: &str) -> bool {
    !value.is_empty()
        && value != "."
        && value != ".."
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"-_.".contains(&byte))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::process::Command;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    static NEXT_TEMP_ID: AtomicU64 = AtomicU64::new(0);

    struct TempGitRoot(PathBuf);

    impl TempGitRoot {
        fn new() -> Self {
            let nonce = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos();
            let id = NEXT_TEMP_ID.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "asn-git-resolution-{}-{nonce}-{id}",
                std::process::id()
            ));
            fs::create_dir_all(&path).expect("create isolated temporary Git test root");
            Self(path)
        }
    }

    impl Drop for TempGitRoot {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn git_in(path: &Path, args: &[&str]) {
        let mut command = Command::new("git");
        command
            .arg("-C")
            .arg(path)
            .args(args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            command.creation_flags(0x0800_0000);
        }
        let output = command.output().expect("run Git in temp repository");
        assert!(
            output.status.success(),
            "git {:?} failed: {}",
            args,
            String::from_utf8_lossy(&output.stderr)
        );
    }

    fn create_repo(root: &Path) -> PathBuf {
        let primary = root.join("primary checkout");
        git_in(
            root,
            &[
                "-c",
                "init.defaultBranch=main",
                "init",
                "--template",
                root.to_str().unwrap(),
                primary.to_str().unwrap(),
            ],
        );
        git_in(&primary, &["config", "user.name", "Repository Test"]);
        git_in(
            &primary,
            &["config", "user.email", "repo-test@example.invalid"],
        );
        git_in(&primary, &["config", "commit.gpgsign", "false"]);
        git_in(
            &primary,
            &[
                "config",
                "core.hooksPath",
                root.join("no-hooks").to_str().unwrap(),
            ],
        );
        git_in(
            &primary,
            &[
                "remote",
                "add",
                "origin",
                "https://github.com/example/project.git",
            ],
        );
        git_in(&primary, &["config", "branch.main.remote", "origin"]);
        fs::write(primary.join("README.md"), "repository resolver fixture\n")
            .expect("write fixture commit content");
        git_in(&primary, &["add", "README.md"]);
        git_in(&primary, &["commit", "-m", "initial fixture commit"]);
        primary
    }

    #[test]
    fn parses_supported_github_remotes_and_rejects_credentialed_or_ambiguous_urls() {
        for value in [
            "https://github.com/owner/repo.git",
            "HTTPS://GitHub.com/owner/repo",
            "git@github.com:owner/repo.git",
            "ssh://git@github.com/owner/repo.git",
        ] {
            assert_eq!(
                parse_github_remote(value),
                Some(("owner".into(), "repo".into())),
                "remote: {value}"
            );
        }
        for value in [
            "https://user:secret@github.com/owner/repo.git",
            "https://github.com:443/owner/repo.git",
            "https://github.com/owner/repo/extra.git",
            "https://github.com/owner/../repo.git",
            "https://github.com/owner/repo.git?token=x",
            "https://github.enterprise.com/owner/repo.git",
        ] {
            assert_eq!(parse_github_remote(value), None, "{value}");
        }
    }

    #[test]
    fn resolves_explicit_repository_and_linked_worktree_common_identity() {
        let temp = TempGitRoot::new();
        let primary_path = create_repo(&temp.0);
        let primary = resolve_repository(primary_path.to_str().unwrap())
            .expect("resolve the explicitly selected repository");

        assert_eq!(primary.branch, "main");
        assert_eq!(primary.owner, "example");
        assert_eq!(primary.name, "project");
        assert_eq!(primary.remote_url, "https://github.com/example/project");
        assert_eq!(
            primary.canonical_path,
            primary_path
                .canonicalize()
                .unwrap()
                .to_string_lossy()
                .into_owned()
        );
        assert_eq!(primary.common_dir, primary.worktree_dir);
        assert_eq!(primary.head_sha.len(), 40);

        let linked_path = temp.0.join("linked worktree");
        git_in(
            &primary_path,
            &[
                "worktree",
                "add",
                "-b",
                "feature/linked",
                linked_path.to_str().unwrap(),
            ],
        );
        let linked = resolve_repository(linked_path.to_str().unwrap())
            .expect("resolve an explicitly selected linked worktree");

        assert_eq!(linked.branch, "feature/linked");
        assert_eq!(linked.common_dir, primary.common_dir);
        assert_ne!(linked.worktree_dir, primary.worktree_dir);
        assert_eq!(linked.owner, primary.owner);
        assert_eq!(linked.name, primary.name);
        assert_eq!(
            linked.canonical_path,
            linked_path
                .canonicalize()
                .unwrap()
                .to_string_lossy()
                .into_owned()
        );
    }
}
