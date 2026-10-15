//! Explicit, read-first GitHub PR and current-commit check refresh via `gh`.
//!
//! A caller must provide the selected repository path and opt in to the user's
//! existing GitHub CLI login. A successful explicit connection starts one
//! quiet five-minute poller for that selected worktree; disconnect, reselection,
//! and pause prevent later polls from publishing results.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use tauri::{AppHandle, Emitter, Manager};

use crate::broker::reducer::BrokerUpdate;
use crate::broker::types::{
    ConfigurationState, ConnectionHealth, Integration, IntegrationErrorKind, IntegrationProvider,
    OpaqueId, Repository, SchemaVersion, SourceKind, SourceScopedId,
};
use crate::repositories::{self, RepositoryIdentity};

const GH_TIMEOUT: Duration = Duration::from_secs(12);
const TOTAL_GH_TIMEOUT: Duration = Duration::from_secs(35);
const MAX_TOTAL_OUTPUT: usize = 4 * 1024 * 1024;
const NORMAL_REFRESH_INTERVAL: Duration = Duration::from_secs(45);
const MAX_BACKOFF: Duration = Duration::from_secs(15 * 60);
const MAX_REMOTES: usize = 8;
const MAX_CACHE_ENTRIES: usize = 16;
const MAX_STABLE_IDENTITIES: usize = 64;
const POLL_INTERVAL: Duration = Duration::from_secs(5 * 60);

#[derive(Clone, Copy, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RefreshOptions {
    /// Use the account already authenticated by the local `gh` executable.
    pub use_gh: bool,
    /// Bypass the normal refresh interval. Rate limit and failure backoff still apply.
    #[serde(default)]
    pub force: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PullRequestSnapshot {
    pub number: u64,
    pub title: String,
    pub state: String,
    pub is_draft: bool,
    pub url: String,
    pub head_owner: String,
    pub head_repo: String,
    pub head_ref: String,
    pub head_sha: String,
    pub base_ref: String,
    pub review_decision: Option<String>,
    pub head_matches: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum CheckKind {
    CheckRun,
    CommitStatus,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum CheckState {
    Pending,
    InProgress,
    Passed,
    Failed,
    Skipped,
    Neutral,
    Cancelled,
    Unknown,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CheckSnapshot {
    pub id: String,
    pub name: String,
    pub kind: CheckKind,
    pub status: CheckState,
    pub conclusion: Option<String>,
    pub url: Option<String>,
    pub head_sha: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum GithubHealth {
    Ready,
    GhUnavailable,
    Unauthenticated,
    Offline,
    RateLimited,
    PermissionDenied,
    Error,
    NotConnected,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GithubSnapshot {
    pub repository: RepositoryIdentity,
    pub pull_request: Option<PullRequestSnapshot>,
    pub checks: Vec<CheckSnapshot>,
    pub health: GithubHealth,
    pub fetched_at_unix_ms: Option<u64>,
    pub stale: bool,
    /// Safe user-facing message only. Process output and credentials are never returned.
    pub error: Option<String>,
}

#[derive(Clone, Default)]
pub struct GithubService {
    cache: Arc<Mutex<HashMap<String, CacheEntry>>>,
    identities: Arc<Mutex<StableIdentities>>,
    scheduled: Arc<Mutex<HashMap<String, Arc<AtomicBool>>>>,
}

#[derive(Default)]
struct StableIdentities {
    repositories: HashMap<String, String>,
    worktrees: HashMap<String, String>,
    next_id: u64,
}

impl StableIdentities {
    fn assign(
        map: &mut HashMap<String, String>,
        path: &str,
        next_id: &mut u64,
        prefix: &str,
    ) -> Option<String> {
        if let Some(id) = map.get(path) {
            return Some(id.clone());
        }
        if map.len() >= MAX_STABLE_IDENTITIES {
            return None;
        }
        *next_id = next_id.saturating_add(1);
        let id = format!("{prefix}-{}", *next_id);
        map.insert(path.to_string(), id.clone());
        Some(id)
    }
}

#[derive(Clone)]
struct CacheEntry {
    snapshot: GithubSnapshot,
    next_allowed: std::time::Instant,
    failure_count: u32,
}

impl GithubService {
    /// Resolve a selected checkout and, only when requested, refresh it using
    /// the user's existing `gh` executable credentials.
    pub async fn refresh(
        &self,
        repo_path: String,
        options: RefreshOptions,
    ) -> Result<GithubSnapshot, String> {
        // The synchronous process calls are bounded, but run them outside the
        // async executor so they do not stall the Tauri runtime.
        let service = self.clone();
        tauri::async_runtime::spawn_blocking(move || service.refresh_blocking(&repo_path, options))
            .await
            .map_err(|_| "GitHub refresh could not be started.".to_string())?
    }

    fn refresh_blocking(
        &self,
        repo_path: &str,
        options: RefreshOptions,
    ) -> Result<GithubSnapshot, String> {
        if !options.use_gh {
            self.stop_all_schedulers();
        }
        let mut repository =
            repositories::resolve_repository(repo_path).map_err(|error| error.to_string())?;
        if options.use_gh {
            self.stop_schedulers_except(&repository.worktree_dir);
        }
        {
            let mut identities = self
                .identities
                .lock()
                .map_err(|_| "Repository identity state is unavailable.".to_string())?;
            let repo_path = repository.common_dir.clone();
            let worktree_path = repository.worktree_dir.clone();
            let StableIdentities {
                repositories,
                worktrees,
                next_id,
            } = &mut *identities;
            repository.repository_id = Some(
                StableIdentities::assign(repositories, &repo_path, next_id, "local-repository")
                    .ok_or_else(|| {
                        "Too many repositories have been selected in this session.".to_string()
                    })?,
            );
            repository.worktree_id = Some(
                StableIdentities::assign(worktrees, &worktree_path, next_id, "local-worktree")
                    .ok_or_else(|| {
                        "Too many worktrees have been selected in this session.".to_string()
                    })?,
            );
        }
        let key = repository.worktree_id.clone().unwrap_or_default();
        let mut cache = self
            .cache
            .lock()
            .map_err(|_| "GitHub cache is unavailable.".to_string())?;
        let cached = cache.get(&key).cloned();

        if !options.use_gh {
            return Ok(retained_snapshot(
                repository,
                cached,
                GithubHealth::NotConnected,
                "Connect GitHub with the installed GitHub CLI to load pull requests and checks.",
            ));
        }
        if let Some(entry) = &cached {
            let same_identity = entry.snapshot.repository == repository;
            let blocked_by_failure =
                entry.failure_count > 0 && std::time::Instant::now() < entry.next_allowed;
            let blocked_by_normal_interval = !options.force
                && entry.failure_count == 0
                && std::time::Instant::now() < entry.next_allowed;
            if blocked_by_failure || blocked_by_normal_interval {
                return Ok(cached_for_repository(
                    &entry.snapshot,
                    repository,
                    !same_identity || entry.failure_count > 0,
                ));
            }
            if !options.force && same_identity && entry.snapshot.health == GithubHealth::Ready {
                let age = entry
                    .snapshot
                    .fetched_at_unix_ms
                    .and_then(|time| unix_time_ms().checked_sub(time))
                    .unwrap_or(u64::MAX);
                if age < NORMAL_REFRESH_INTERVAL.as_millis() as u64 {
                    return Ok(cached_for_repository(&entry.snapshot, repository, false));
                }
            }
        }

        let result = fetch_snapshot(&repository);
        match result {
            Ok(mut snapshot) => {
                snapshot.fetched_at_unix_ms = Some(unix_time_ms());
                snapshot.health = GithubHealth::Ready;
                snapshot.stale = false;
                snapshot.error = None;
                insert_cache(
                    &mut cache,
                    key,
                    CacheEntry {
                        snapshot: snapshot.clone(),
                        next_allowed: std::time::Instant::now() + NORMAL_REFRESH_INTERVAL,
                        failure_count: 0,
                    },
                );
                Ok(snapshot)
            }
            Err((health, message, retry_after)) => {
                if let Some(mut entry) = cached {
                    entry.failure_count = entry.failure_count.saturating_add(1);
                    let exp = 2u64.saturating_pow(entry.failure_count.min(8));
                    let backoff = retry_after.unwrap_or_else(|| {
                        let base = (15 * exp).min(MAX_BACKOFF.as_secs());
                        Duration::from_secs(base.saturating_add(unix_time_ms() % 11))
                    });
                    entry.next_allowed = std::time::Instant::now() + backoff;
                    entry.snapshot = cached_for_repository(&entry.snapshot, repository, true);
                    entry.snapshot.health = health;
                    entry.snapshot.error = Some(message);
                    let snapshot = entry.snapshot.clone();
                    cache.insert(key, entry);
                    Ok(snapshot)
                } else {
                    let snapshot = GithubSnapshot {
                        repository,
                        pull_request: None,
                        checks: Vec::new(),
                        health,
                        fetched_at_unix_ms: None,
                        stale: true,
                        error: Some(message),
                    };
                    insert_cache(
                        &mut cache,
                        key,
                        CacheEntry {
                            snapshot: snapshot.clone(),
                            next_allowed: std::time::Instant::now()
                                + retry_after.unwrap_or(Duration::from_secs(30)),
                            failure_count: 1,
                        },
                    );
                    Ok(snapshot)
                }
            }
        }
    }

    /// Start quiet five-minute refreshes after a successful, explicit connect.
    /// The caller must invoke this only following a user initiated refresh with
    /// `useGh=true` and a Ready result.
    pub fn start_scheduler(&self, app: AppHandle, repo_path: String) -> Result<(), String> {
        let repository =
            repositories::resolve_repository(&repo_path).map_err(|error| error.to_string())?;
        let key = repository.worktree_dir.clone();
        let stop = Arc::new(AtomicBool::new(false));
        {
            let mut schedules = self
                .scheduled
                .lock()
                .map_err(|_| "GitHub scheduler is unavailable.".to_string())?;
            if schedules.contains_key(&key) {
                return Ok(());
            }
            // Only one explicitly selected checkout polls at a time. A new
            // selection invalidates the prior loop before it can publish.
            for stop in schedules.values() {
                stop.store(true, Ordering::Relaxed);
            }
            schedules.clear();
            schedules.insert(key.clone(), stop.clone());
        }
        let service = self.clone();
        tauri::async_runtime::spawn(async move {
            loop {
                tokio::time::sleep(POLL_INTERVAL).await;
                if stop.load(Ordering::Relaxed) {
                    break;
                }
                let is_paused = {
                    let Some(shared) = app.try_state::<crate::Shared>() else {
                        break;
                    };
                    shared.paused.load(Ordering::Relaxed)
                };
                if is_paused {
                    continue;
                }
                let result = service
                    .refresh(
                        repo_path.clone(),
                        RefreshOptions {
                            use_gh: true,
                            force: false,
                        },
                    )
                    .await;
                if stop.load(Ordering::Relaxed) {
                    break;
                }
                if app
                    .try_state::<crate::Shared>()
                    .is_some_and(|shared| shared.paused.load(Ordering::Relaxed))
                {
                    continue;
                }
                if let Ok(snapshot) = result {
                    // Hold the scheduler registry lock across publication. A
                    // concurrent disconnect/reselection cannot register a
                    // new generation between the check and emit, allowing an
                    // old worker to overwrite the newly selected repository.
                    if let Ok(schedules) = service.scheduled.lock() {
                        let current = schedules
                            .get(&key)
                            .is_some_and(|registered| Arc::ptr_eq(registered, &stop));
                        let paused = app
                            .try_state::<crate::Shared>()
                            .is_some_and(|shared| shared.paused.load(Ordering::Relaxed));
                        if current && !paused && !stop.load(Ordering::Relaxed) {
                            if let Some(shared) = app.try_state::<crate::Shared>() {
                                for update in broker_updates(&snapshot) {
                                    if shared.paused.load(Ordering::Relaxed)
                                        || stop.load(Ordering::Relaxed)
                                    {
                                        break;
                                    }
                                    let _ = crate::publish_broker_update(
                                        &app,
                                        &shared,
                                        update,
                                        unix_time_ms(),
                                        None,
                                        None,
                                    );
                                }
                                if !shared.paused.load(Ordering::Relaxed)
                                    && !stop.load(Ordering::Relaxed)
                                {
                                    crate::publish_github_notification_policy(
                                        &app,
                                        &shared,
                                        &snapshot,
                                        unix_time_ms(),
                                    );
                                    let _ = app.emit_to(
                                        crate::island::WINDOW_LABEL,
                                        "github-snapshot",
                                        snapshot,
                                    );
                                }
                            }
                        }
                    }
                }
                if stop.load(Ordering::Relaxed) {
                    break;
                }
            }
            if let Ok(mut schedules) = service.scheduled.lock() {
                remove_schedule_if_current(&mut schedules, &key, &stop);
            }
        });
        Ok(())
    }

    /// Stop the scheduler for a selected worktree. Used by the explicit
    /// `useGh=false` disconnect path.
    pub fn stop_all_schedulers(&self) {
        if let Ok(mut schedules) = self.scheduled.lock() {
            for stop in schedules.values() {
                stop.store(true, Ordering::Relaxed);
            }
            schedules.clear();
        }
    }

    fn stop_schedulers_except(&self, selected_worktree: &str) {
        if let Ok(mut schedules) = self.scheduled.lock() {
            let obsolete = schedules
                .keys()
                .filter(|key| key.as_str() != selected_worktree)
                .cloned()
                .collect::<Vec<_>>();
            for key in obsolete {
                if let Some(stop) = schedules.remove(&key) {
                    stop.store(true, Ordering::Relaxed);
                }
            }
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct GhPullRequest {
    number: u64,
    title: String,
    state: String,
    #[serde(default)]
    is_draft: bool,
    url: String,
    head_ref_name: String,
    head_ref_oid: String,
    head_repository_owner: Option<GhOwner>,
    head_repository: Option<GhRepo>,
    base_ref_name: String,
    review_decision: Option<String>,
}

#[derive(Deserialize)]
struct GhOwner {
    login: String,
}

#[derive(Deserialize)]
struct GhRepo {
    name: String,
}

fn fetch_snapshot(
    repository: &RepositoryIdentity,
) -> Result<GithubSnapshot, (GithubHealth, String, Option<Duration>)> {
    if repository.github_remote_repositories.len() > MAX_REMOTES {
        return Err((
            GithubHealth::Error,
            "This repository has too many GitHub remotes to resolve safely.".into(),
            None,
        ));
    }
    let mut budget = FetchBudget::new();
    let mut pull_requests = Vec::new();
    let mut seen_pull_requests = Vec::new();
    let mut visited = Vec::<String>::new();
    for (owner, name) in repository
        .github_remote_repositories
        .iter()
        .take(MAX_REMOTES)
    {
        let repo = format!("{owner}/{name}");
        if visited.iter().any(|seen| seen == &repo) {
            continue;
        }
        visited.push(repo.clone());
        // `gh pr list --head` accepts a branch name but does not support the
        // owner:branch form, so owner/repository identity is checked in JSON.
        let head_selector = repository.branch.as_str();
        let args = [
            "pr", "list", "--repo", repo.as_str(), "--head", head_selector, "--state", "all", "--limit", "100",
            "--json", "number,title,state,isDraft,url,headRefName,headRefOid,headRepositoryOwner,headRepository,baseRefName,reviewDecision",
        ];
        let output = run_gh(&args, &mut budget)?;
        let rows: Vec<GhPullRequest> = serde_json::from_slice(&output).map_err(|_| {
            (
                GithubHealth::Error,
                "GitHub returned an unreadable pull request response.".into(),
                None,
            )
        })?;
        if rows.len() >= 100 {
            return Err((
                GithubHealth::Error,
                "This branch has too many matching pull requests to resolve safely.".into(),
                None,
            ));
        }
        for row in rows {
            if matches_local_head(
                &row,
                &repository.owner,
                &repository.name,
                &repository.branch,
            ) {
                let dedup_key = (row.number, row.url.clone());
                if seen_pull_requests.contains(&dedup_key) {
                    continue;
                }
                seen_pull_requests.push(dedup_key);
                pull_requests.push(row);
            }
        }
    }
    let exact = pull_requests
        .iter()
        .filter(|row| row.head_ref_oid.eq_ignore_ascii_case(&repository.head_sha))
        .count();
    let candidates = if exact > 0 {
        pull_requests
            .into_iter()
            .filter(|row| row.head_ref_oid.eq_ignore_ascii_case(&repository.head_sha))
            .collect::<Vec<_>>()
    } else {
        pull_requests
    };
    if candidates.len() > 1 {
        return Err((GithubHealth::Error, "Several pull requests match this branch. Select a pull request explicitly in a later version.".into(), None));
    }
    let pull_request = candidates.into_iter().next().map(|row| {
        let head_owner = row
            .head_repository_owner
            .map(|value| value.login)
            .unwrap_or_default();
        let head_repo = row
            .head_repository
            .map(|value| value.name)
            .unwrap_or_default();
        PullRequestSnapshot {
            number: row.number,
            title: row.title.chars().take(300).collect(),
            state: normalize_pr_state(&row.state),
            is_draft: row.is_draft,
            url: safe_github_url(&row.url).unwrap_or_else(|| {
                format!(
                    "https://github.com/{}/{}/pull/{}",
                    repository.owner, repository.name, row.number
                )
            }),
            head_owner,
            head_repo,
            head_ref: row.head_ref_name,
            head_matches: row.head_ref_oid.eq_ignore_ascii_case(&repository.head_sha),
            head_sha: row.head_ref_oid,
            base_ref: row.base_ref_name.chars().take(200).collect(),
            review_decision: row.review_decision,
        }
    });

    let checks_repo = pull_request
        .as_ref()
        .map(|pr| format!("{}/{}", pr.head_owner, pr.head_repo))
        .filter(|repo| !repo.ends_with('/'))
        .unwrap_or_else(|| format!("{}/{}", repository.owner, repository.name));
    let check_path = format!(
        "repos/{checks_repo}/commits/{}/check-runs?per_page=100",
        repository.head_sha
    );
    let check_args = [
        "api",
        "--paginate",
        "--jq",
        ".check_runs[] | {id, name, status, conclusion, html_url, head_sha}",
        check_path.as_str(),
    ];
    let check_output = run_gh(&check_args, &mut budget)?;
    let mut checks = parse_check_lines(&check_output, CheckKind::CheckRun, &repository.head_sha)?;

    let statuses_path = format!(
        "repos/{checks_repo}/commits/{}/statuses?per_page=100",
        repository.head_sha
    );
    let status_args = [
        "api",
        "--paginate",
        "--jq",
        ".[] | {id, context, state, target_url, sha}",
        statuses_path.as_str(),
    ];
    let statuses_output = run_gh(&status_args, &mut budget)?;
    checks.extend(parse_check_lines(
        &statuses_output,
        CheckKind::CommitStatus,
        &repository.head_sha,
    )?);
    checks.sort_by(|left, right| {
        left.name
            .cmp(&right.name)
            .then_with(|| left.id.cmp(&right.id))
    });

    Ok(GithubSnapshot {
        repository: repository.clone(),
        pull_request,
        checks,
        health: GithubHealth::Ready,
        fetched_at_unix_ms: None,
        stale: false,
        error: None,
    })
}

struct FetchBudget {
    deadline: std::time::Instant,
    output_bytes: usize,
}

impl FetchBudget {
    fn new() -> Self {
        Self {
            deadline: std::time::Instant::now() + TOTAL_GH_TIMEOUT,
            output_bytes: 0,
        }
    }
}

fn run_gh(
    args: &[&str],
    budget: &mut FetchBudget,
) -> Result<Vec<u8>, (GithubHealth, String, Option<Duration>)> {
    if std::time::Instant::now() >= budget.deadline {
        return Err((GithubHealth::Offline, "GitHub refresh exceeded its time limit. The last saved result is shown when available.".into(), Some(Duration::from_secs(30))));
    }
    let mut command = std::process::Command::new("gh");
    command
        .args(args)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    // Honor gh's credential store, never ambient tokens that could silently
    // widen this app's credential authority.
    command
        .env_remove("GH_TOKEN")
        .env_remove("GITHUB_TOKEN")
        .env_remove("GH_ENTERPRISE_TOKEN")
        .env_remove("GH_REPO")
        .env("GH_HOST", "github.com");
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000);
    }
    let mut child = command.spawn().map_err(|_| {
        (
            GithubHealth::GhUnavailable,
            "GitHub CLI (gh) is not installed or is unavailable on PATH.".into(),
            None,
        )
    })?;
    let stdout = child.stdout.take().ok_or((
        GithubHealth::Error,
        "GitHub CLI could not be started.".into(),
        None,
    ))?;
    let stderr = child.stderr.take().ok_or((
        GithubHealth::Error,
        "GitHub CLI could not be started.".into(),
        None,
    ))?;
    let stdout_reader = std::thread::spawn(move || read_limit(stdout, 2 * 1024 * 1024));
    let stderr_reader = std::thread::spawn(move || read_limit(stderr, 16 * 1024));
    let deadline = (std::time::Instant::now() + GH_TIMEOUT).min(budget.deadline);
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break Some(status),
            Ok(None) if std::time::Instant::now() < deadline => {
                std::thread::sleep(Duration::from_millis(40))
            }
            Ok(None) => {
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
    let output = stdout_reader.join().unwrap_or_default();
    let diagnostic = stderr_reader.join().unwrap_or_default();
    if output.len() >= 2 * 1024 * 1024 {
        return Err((
            GithubHealth::Error,
            "GitHub returned more data than the safe response limit.".into(),
            None,
        ));
    }
    if std::time::Instant::now() >= deadline {
        return Err((
            GithubHealth::Offline,
            "GitHub did not respond in time. The last saved result is shown when available.".into(),
            Some(Duration::from_secs(30)),
        ));
    }
    if status.is_some_and(|status| status.success()) {
        budget.output_bytes = budget.output_bytes.saturating_add(output.len());
        if budget.output_bytes > MAX_TOTAL_OUTPUT {
            return Err((
                GithubHealth::Error,
                "GitHub returned more data than the safe combined response limit.".into(),
                None,
            ));
        }
        return Ok(output);
    }
    let diagnostic = String::from_utf8_lossy(&diagnostic);
    let (health, message, retry) = classify_diagnostic(&diagnostic);
    Err((health, message, retry))
}

fn read_limit(mut reader: impl std::io::Read, limit: usize) -> Vec<u8> {
    let mut result = Vec::new();
    let mut buffer = [0u8; 8192];
    while result.len() < limit {
        let size = (limit - result.len()).min(buffer.len());
        match reader.read(&mut buffer[..size]) {
            Ok(0) | Err(_) => break,
            Ok(count) => result.extend_from_slice(&buffer[..count]),
        }
    }
    result
}

fn parse_check_lines(
    bytes: &[u8],
    kind: CheckKind,
    local_sha: &str,
) -> Result<Vec<CheckSnapshot>, (GithubHealth, String, Option<Duration>)> {
    let mut checks = Vec::new();
    for line in bytes
        .split(|byte| *byte == b'\n')
        .filter(|line| !line.is_empty())
    {
        let value = serde_json::from_slice::<Value>(line).map_err(|_| {
            (
                GithubHealth::Error,
                "GitHub returned an unreadable check response.".into(),
                None,
            )
        })?;
        if !value.is_object() {
            return Err((
                GithubHealth::Error,
                "GitHub returned an unreadable check response.".into(),
                None,
            ));
        }
        let sha = value
            .get("head_sha")
            .or_else(|| value.get("sha"))
            .and_then(Value::as_str)
            .filter(|sha| !sha.is_empty())
            .ok_or((
                GithubHealth::Error,
                "GitHub returned a check without a commit identity.".into(),
                None,
            ))?;
        if !sha.eq_ignore_ascii_case(local_sha) {
            continue;
        }
        let name = value
            .get("name")
            .or_else(|| value.get("context"))
            .and_then(Value::as_str)
            .unwrap_or("Check");
        let id = value
            .get("id")
            .map(|value| value.to_string())
            .unwrap_or_else(|| name.to_string());
        let status = value.get("status").and_then(Value::as_str);
        let conclusion = value
            .get("conclusion")
            .and_then(Value::as_str)
            .map(str::to_string);
        let status_state = value.get("state").and_then(Value::as_str);
        let state = match kind {
            CheckKind::CheckRun => match (status, conclusion.as_deref()) {
                (Some("queued"), _) => CheckState::Pending,
                (Some("in_progress"), _) => CheckState::InProgress,
                (_, Some(value)) => check_conclusion(value),
                _ => CheckState::Unknown,
            },
            CheckKind::CommitStatus => match status_state {
                Some("pending") => CheckState::Pending,
                Some("success") => CheckState::Passed,
                Some("failure" | "error") => CheckState::Failed,
                _ => CheckState::Unknown,
            },
        };
        checks.push(CheckSnapshot {
            id,
            name: name.chars().take(200).collect(),
            kind,
            status: state,
            conclusion,
            url: value
                .get("html_url")
                .or_else(|| value.get("target_url"))
                .and_then(Value::as_str)
                .and_then(safe_github_url),
            head_sha: local_sha.to_string(),
        });
    }
    Ok(checks)
}

fn matches_local_head(row: &GhPullRequest, owner: &str, name: &str, branch: &str) -> bool {
    row.head_repository_owner
        .as_ref()
        .is_some_and(|value| value.login.eq_ignore_ascii_case(owner))
        && row
            .head_repository
            .as_ref()
            .is_some_and(|value| value.name.eq_ignore_ascii_case(name))
        && row.head_ref_name == branch
}

fn check_conclusion(value: &str) -> CheckState {
    match value {
        "success" => CheckState::Passed,
        "failure" | "timed_out" | "action_required" | "startup_failure" => CheckState::Failed,
        "skipped" => CheckState::Skipped,
        "neutral" => CheckState::Neutral,
        "cancelled" => CheckState::Cancelled,
        _ => CheckState::Unknown,
    }
}

fn safe_github_url(value: &str) -> Option<String> {
    let url = value.strip_prefix("https://")?;
    let (host, path) = url.split_once('/')?;
    if !host.eq_ignore_ascii_case("github.com")
        || path
            .chars()
            .any(|character| matches!(character, '?' | '#' | '@' | '\\'))
    {
        return None;
    }
    Some(format!(
        "https://github.com/{}",
        path.chars().take(1000).collect::<String>()
    ))
}

fn normalize_pr_state(value: &str) -> String {
    match value {
        "MERGED" => "merged",
        "OPEN" => "open",
        "CLOSED" => "closed",
        _ => "unknown",
    }
    .into()
}

fn retained_snapshot(
    repository: RepositoryIdentity,
    cached: Option<CacheEntry>,
    health: GithubHealth,
    error: &str,
) -> GithubSnapshot {
    if let Some(mut entry) = cached {
        entry.snapshot = cached_for_repository(&entry.snapshot, repository, true);
        entry.snapshot.health = health;
        entry.snapshot.error = Some(error.into());
        entry.snapshot
    } else {
        GithubSnapshot {
            repository,
            pull_request: None,
            checks: Vec::new(),
            health,
            fetched_at_unix_ms: None,
            stale: true,
            error: Some(error.into()),
        }
    }
}

fn cached_for_repository(
    snapshot: &GithubSnapshot,
    repository: RepositoryIdentity,
    stale: bool,
) -> GithubSnapshot {
    let same_identity = snapshot.repository == repository;
    let mut snapshot = snapshot.clone();
    snapshot.repository = repository;
    snapshot.stale = stale;
    if let Some(pr) = &mut snapshot.pull_request {
        pr.head_matches = same_identity
            && pr
                .head_sha
                .eq_ignore_ascii_case(&snapshot.repository.head_sha);
    }
    snapshot
}

fn insert_cache(cache: &mut HashMap<String, CacheEntry>, key: String, entry: CacheEntry) {
    if !cache.contains_key(&key) && cache.len() >= MAX_CACHE_ENTRIES {
        if let Some(oldest) = cache.keys().next().cloned() {
            cache.remove(&oldest);
        }
    }
    cache.insert(key, entry);
}

fn remove_schedule_if_current(
    schedules: &mut HashMap<String, Arc<AtomicBool>>,
    key: &str,
    stop: &Arc<AtomicBool>,
) -> bool {
    if schedules
        .get(key)
        .is_some_and(|current| Arc::ptr_eq(current, stop))
    {
        schedules.remove(key);
        true
    } else {
        false
    }
}

#[cfg(test)]
fn schedule_is_current(
    schedules: &HashMap<String, Arc<AtomicBool>>,
    key: &str,
    stop: &Arc<AtomicBool>,
) -> bool {
    schedules
        .get(key)
        .is_some_and(|current| Arc::ptr_eq(current, stop))
}

fn unix_time_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|duration| u64::try_from(duration.as_millis()).ok())
        .unwrap_or_default()
}

/// Project only safe, normalized repository and connection health into the
/// broker. Local paths, remote URLs, PR content and check details stay in the
/// typed GitHub snapshot and never enter persisted broker metadata.
pub fn broker_updates(snapshot: &GithubSnapshot) -> Vec<BrokerUpdate> {
    let repository = &snapshot.repository;
    let repo_id = repository
        .repository_id
        .as_ref()
        .and_then(|id| OpaqueId::new(id.clone()).ok());
    let worktree_id = repository
        .worktree_id
        .as_ref()
        .and_then(|id| OpaqueId::new(id.clone()).ok());
    let mut updates = Vec::new();
    if let (Some(repo_id), Some(worktree_id)) = (repo_id, worktree_id) {
        updates.push(BrokerUpdate::UpsertRepository(Repository {
            schema_version: SchemaVersion::current(),
            identity: SourceScopedId::new(SourceKind::LocalGit, repo_id),
            host: Some("github.com".into()),
            owner: Some(repository.owner.clone()),
            name: Some(repository.name.clone()),
            worktree_id: Some(worktree_id),
            branch: Some(repository.branch.clone()),
            head_sha: Some(repository.head_sha.clone()),
            observed_at_unix_ms: Some(unix_time_ms()),
        }));
    }
    let (connection, last_error) = match snapshot.health {
        GithubHealth::Ready => (ConnectionHealth::Healthy, None),
        GithubHealth::Offline => (
            ConnectionHealth::Offline,
            Some(IntegrationErrorKind::Network),
        ),
        GithubHealth::Unauthenticated => (
            ConnectionHealth::Degraded,
            Some(IntegrationErrorKind::Authentication),
        ),
        GithubHealth::PermissionDenied => (
            ConnectionHealth::Degraded,
            Some(IntegrationErrorKind::Permission),
        ),
        GithubHealth::RateLimited => (
            ConnectionHealth::Degraded,
            Some(IntegrationErrorKind::RateLimited),
        ),
        GithubHealth::NotConnected => (ConnectionHealth::Unknown, None),
        GithubHealth::GhUnavailable | GithubHealth::Error => (
            ConnectionHealth::Degraded,
            Some(IntegrationErrorKind::Other),
        ),
    };
    if let Ok(identity) = OpaqueId::new("integration_github_workflow") {
        updates.push(BrokerUpdate::UpsertIntegration(Integration {
            schema_version: SchemaVersion::current(),
            identity,
            provider: IntegrationProvider::GitHub,
            configuration: if snapshot.health == GithubHealth::NotConnected {
                ConfigurationState::Unconfigured
            } else {
                ConfigurationState::Configured
            },
            connection,
            last_success_at_unix_ms: snapshot.fetched_at_unix_ms,
            data_revision: None,
            retry_at_unix_ms: None,
            last_error,
            unread_event_ids: Vec::new(),
        }));
    }
    updates
}

fn classify_diagnostic(text: &str) -> (GithubHealth, String, Option<Duration>) {
    let text = text.to_ascii_lowercase();
    if text.contains("rate limit") || text.contains("secondary rate") {
        (
            GithubHealth::RateLimited,
            "GitHub is rate limiting requests. The last saved result is shown when available."
                .into(),
            rate_retry_after(&text).or(Some(Duration::from_secs(60))),
        )
    } else if text.contains("not logged in")
        || text.contains("authentication")
        || text.contains("401")
    {
        (
            GithubHealth::Unauthenticated,
            "Sign in with `gh auth login` to connect GitHub.".into(),
            None,
        )
    } else if text.contains("403")
        || text.contains("404")
        || text.contains("permission")
        || text.contains("forbidden")
    {
        (
            GithubHealth::PermissionDenied,
            "GitHub denied access to this repository. Check repository access in `gh auth status`."
                .into(),
            None,
        )
    } else if text.contains("resolve host")
        || text.contains("connection")
        || text.contains("network")
        || text.contains("timed out")
    {
        (
            GithubHealth::Offline,
            "GitHub is unreachable. The last saved result is shown when available.".into(),
            Some(Duration::from_secs(30)),
        )
    } else {
        (
            GithubHealth::Error,
            "GitHub refresh failed. Check `gh auth status` and try again.".into(),
            Some(Duration::from_secs(30)),
        )
    }
}

fn rate_retry_after(diagnostic: &str) -> Option<Duration> {
    for marker in ["retry after ", "retry-after: "] {
        if let Some(index) = diagnostic.find(marker) {
            let rest = &diagnostic[index + marker.len()..];
            let seconds = rest
                .split(|character: char| !character.is_ascii_digit())
                .next()?
                .parse::<u64>()
                .ok()?;
            return Some(Duration::from_secs(seconds));
        }
    }
    if let Some(index) = diagnostic.find("reset at ") {
        let rest = &diagnostic[index + 9..];
        let reset = rest
            .split(|character: char| !character.is_ascii_digit())
            .next()?
            .parse::<u64>()
            .ok()?;
        let now = SystemTime::now().duration_since(UNIX_EPOCH).ok()?.as_secs();
        return Some(Duration::from_secs(reset.saturating_sub(now)));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn check_run_and_status_mappings_keep_old_sha_results_out() {
        let lines = json!({"id": 1, "name": "unit", "status": "completed", "conclusion": "success", "html_url": "https://github.com/o/r/actions/runs/1", "head_sha": "new"}).to_string();
        let stale = json!({"id": 2, "name": "old", "status": "completed", "conclusion": "success", "head_sha": "old"}).to_string();
        let body = format!("{lines}\n{stale}\n");
        let parsed = parse_check_lines(body.as_bytes(), CheckKind::CheckRun, "new").unwrap();
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].status, CheckState::Passed);
        assert_eq!(parsed[0].head_sha, "new");

        let status = json!({"id": 4, "context": "ci", "state": "error", "target_url": "https://github.com/o/r/check", "sha": "new"}).to_string();
        let parsed = parse_check_lines(status.as_bytes(), CheckKind::CommitStatus, "new").unwrap();
        assert_eq!(parsed[0].status, CheckState::Failed);
    }

    #[test]
    fn changed_head_keeps_cached_checks_stale_and_unmatched() {
        let original = RepositoryIdentity {
            canonical_path: "C:/repo".into(),
            common_dir: "C:/repo/.git".into(),
            worktree_dir: "C:/repo/.git".into(),
            branch: "feature/x".into(),
            head_sha: "old-sha".into(),
            remote_name: "origin".into(),
            remote_url: "https://github.com/owner/repo".into(),
            owner: "owner".into(),
            name: "repo".into(),
            repository_id: Some("local-repository-1".into()),
            worktree_id: Some("local-worktree-1".into()),
            github_remote_repositories: vec![("owner".into(), "repo".into())],
        };
        let mut current = original.clone();
        current.head_sha = "new-sha".into();
        let cached = GithubSnapshot {
            repository: original,
            pull_request: Some(PullRequestSnapshot {
                number: 1,
                title: "title".into(),
                state: "open".into(),
                is_draft: false,
                url: "https://github.com/owner/repo/pull/1".into(),
                head_owner: "owner".into(),
                head_repo: "repo".into(),
                head_ref: "feature/x".into(),
                head_sha: "old-sha".into(),
                base_ref: "main".into(),
                review_decision: None,
                head_matches: true,
            }),
            checks: vec![CheckSnapshot {
                id: "1".into(),
                name: "old green".into(),
                kind: CheckKind::CheckRun,
                status: CheckState::Passed,
                conclusion: Some("success".into()),
                url: None,
                head_sha: "old-sha".into(),
            }],
            health: GithubHealth::Ready,
            fetched_at_unix_ms: Some(1),
            stale: false,
            error: None,
        };
        let snapshot = cached_for_repository(&cached, current, true);
        assert!(snapshot.stale);
        assert!(!snapshot.pull_request.unwrap().head_matches);
        assert_eq!(snapshot.checks[0].head_sha, "old-sha");
    }

    #[test]
    fn broker_projection_uses_opaque_ids_and_excludes_local_paths() {
        let snapshot = GithubSnapshot {
            repository: RepositoryIdentity {
                canonical_path: "C:/private/project".into(),
                common_dir: "C:/private/project/.git".into(),
                worktree_dir: "C:/private/project/.git".into(),
                branch: "main".into(),
                head_sha: "abc".into(),
                remote_name: "origin".into(),
                remote_url: "https://github.com/owner/repo".into(),
                owner: "owner".into(),
                name: "repo".into(),
                repository_id: Some("local-repository-1".into()),
                worktree_id: Some("local-worktree-1".into()),
                github_remote_repositories: vec![("owner".into(), "repo".into())],
            },
            pull_request: None,
            checks: Vec::new(),
            health: GithubHealth::Offline,
            fetched_at_unix_ms: Some(10),
            stale: true,
            error: Some("generic".into()),
        };
        let serialized = serde_json::to_string(&broker_updates(&snapshot)).unwrap();
        assert!(serialized.contains("local-repository-1"));
        assert!(serialized.contains("local-worktree-1"));
        assert!(serialized.contains("offline"));
        assert!(!serialized.contains("C:/private/project"));
        assert!(!serialized.contains("https://github.com"));
    }

    #[test]
    fn pull_request_matching_includes_the_fork_repository_name() {
        let make = |name: &str| GhPullRequest {
            number: 7,
            title: "PR".into(),
            state: "OPEN".into(),
            is_draft: false,
            url: "https://github.com/upstream/repo/pull/7".into(),
            head_ref_name: "feature/x".into(),
            head_ref_oid: "sha".into(),
            head_repository_owner: Some(GhOwner {
                login: "owner".into(),
            }),
            head_repository: Some(GhRepo { name: name.into() }),
            base_ref_name: "main".into(),
            review_decision: None,
        };
        assert!(matches_local_head(
            &make("repo"),
            "owner",
            "repo",
            "feature/x"
        ));
        assert!(!matches_local_head(
            &make("other"),
            "owner",
            "repo",
            "feature/x"
        ));
    }

    #[test]
    fn malformed_paginated_check_rows_are_reported_as_invalid() {
        let result = parse_check_lines(b"not-json\n", CheckKind::CheckRun, "sha");
        assert_eq!(result.unwrap_err().0, GithubHealth::Error);
        let missing_sha = br#"{"id":1,"name":"build","status":"completed","conclusion":"success"}"#;
        let result = parse_check_lines(missing_sha, CheckKind::CheckRun, "sha");
        assert_eq!(result.unwrap_err().0, GithubHealth::Error);
    }

    #[test]
    fn rate_limit_and_permission_failures_have_bounded_user_messages() {
        assert_eq!(
            classify_diagnostic("API rate limit exceeded").0,
            GithubHealth::RateLimited
        );
        assert_eq!(
            classify_diagnostic("HTTP 403 Forbidden").0,
            GithubHealth::PermissionDenied
        );
        assert_eq!(
            classify_diagnostic("could not resolve host").0,
            GithubHealth::Offline
        );
        assert!(!classify_diagnostic("private token: abc").1.contains("abc"));
        assert_eq!(
            rate_retry_after("retry after 3600"),
            Some(Duration::from_secs(3600))
        );
        let actual = rate_retry_after("rate limit reset at 4102444800")
            .unwrap()
            .as_secs();
        let expected = 4102444800u64.saturating_sub(unix_time_ms() / 1000);
        assert!(actual <= expected && expected - actual <= 1);
    }

    #[test]
    fn finishing_old_scheduler_cannot_remove_reconnected_generation() {
        let old = Arc::new(AtomicBool::new(true));
        let current = Arc::new(AtomicBool::new(false));
        let mut schedules = HashMap::from([("worktree".to_string(), current.clone())]);
        assert!(!schedule_is_current(&schedules, "worktree", &old));
        assert!(schedule_is_current(&schedules, "worktree", &current));
        assert!(!remove_schedule_if_current(
            &mut schedules,
            "worktree",
            &old
        ));
        assert!(Arc::ptr_eq(schedules.get("worktree").unwrap(), &current));
        assert!(remove_schedule_if_current(
            &mut schedules,
            "worktree",
            &current
        ));
        assert!(!schedules.contains_key("worktree"));
    }
}
