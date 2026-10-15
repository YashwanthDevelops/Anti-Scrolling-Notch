//! Backend-owned notification priority and deduplication policy.
//!
//! Native toast activation is deliberately unavailable until the app has a
//! verified Windows App SDK registration and activation route. This module
//! still provides deterministic island badge/sound events and safe toast XML
//! formatting for a future verified adapter.

use std::collections::HashMap;

use serde::Serialize;

use crate::broker::stream::BrokerSnapshot;
use crate::broker::types::{
    ConnectionHealth, IntegrationProvider, SessionActivity, SessionLifecycle, TurnLifecycle,
    TurnOutcome, WaitingState,
};

const DEDUP_CAPACITY: usize = 4096;
const DEDUP_TTL_MS: u64 = 24 * 60 * 60 * 1000;
const RECENT_COMPLETION_MS: u64 = 10 * 60 * 1000;
const ACTIONABLE_FAILURE_MS: u64 = 60 * 60 * 1000;
const SAME_TARGET_COOLDOWN_MS: u64 = 15 * 1000;
const MAX_TRIGGERS_PER_UPDATE: usize = 64;

/// Stable app identity reserved for a future registered notification adapter.
/// It is not registered with Windows by this module.
pub const APP_AUMID: &str = "YashwanthDevelops.AntiScrollingNotch";

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum NotificationPriority {
    Waiting,
    Failure,
    Active,
    Completion,
    #[default]
    Idle,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum NotificationCategory {
    Waiting,
    Failure,
    Active,
    Completion,
    GitHub,
    Health,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum NotificationTarget {
    /// Opens the verified generic Codex app route; it does not identify a chat.
    OpenCodex,
    /// Exact canonical public GitHub URL. No arbitrary URL is accepted.
    OpenGitHub { url: String },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NotificationTrigger {
    pub id: String,
    pub category: NotificationCategory,
    pub priority: NotificationPriority,
    pub title: String,
    pub detail: String,
    pub target: Option<NotificationTarget>,
    pub occurred_at_unix_ms: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SoundCue {
    Attention,
    Failure,
    Activity,
    Completion,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum NativeToastStatus {
    Disabled,
    Unavailable,
    SubmittedUnverified,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NotificationUpdate {
    pub primary: NotificationPriority,
    pub badge_count: u32,
    pub sound: Option<SoundCue>,
    pub triggers: Vec<NotificationTrigger>,
    pub native_toast: NativeToastStatus,
    pub suppressed: bool,
}

/// Runtime policy input. Defaults are quiet, opt-in safe, and do not enable
/// unverified native toast delivery.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NotificationPreferences {
    pub paused: bool,
    pub quiet: bool,
    pub fullscreen: bool,
    pub sound_enabled: bool,
    pub native_toast_enabled: bool,
    pub waiting_enabled: bool,
    pub failure_enabled: bool,
    pub active_enabled: bool,
    pub completion_enabled: bool,
    pub github_enabled: bool,
    pub health_enabled: bool,
}

impl Default for NotificationPreferences {
    fn default() -> Self {
        Self {
            paused: false,
            quiet: true,
            fullscreen: false,
            sound_enabled: true,
            native_toast_enabled: false,
            waiting_enabled: true,
            failure_enabled: true,
            active_enabled: true,
            completion_enabled: true,
            github_enabled: true,
            health_enabled: true,
        }
    }
}

#[derive(Clone, Debug)]
struct SeenRevision {
    revision: String,
    observed_at_ms: u64,
    ordinal: u64,
}

/// Stateful deduplication for one app lifetime. The map is bounded and entries
/// expire after 24 hours. The first broker snapshot establishes a silent
/// baseline so reloads do not announce every existing turn or state.
#[derive(Default)]
pub struct NotificationService {
    seen: HashMap<String, SeenRevision>,
    last_emitted: HashMap<String, u64>,
    github: HashMap<String, GithubBaseline>,
    last_priority: NotificationPriority,
    last_badge_count: u32,
    ordinal: u64,
    initialized: bool,
}

#[derive(Clone, Debug)]
struct GithubBaseline {
    head_sha: String,
    checks: HashMap<String, String>,
    observed_at_ms: u64,
}

impl NotificationService {
    pub fn process_snapshot(
        &mut self,
        snapshot: &BrokerSnapshot,
        now_unix_ms: u64,
        preferences: NotificationPreferences,
    ) -> NotificationUpdate {
        self.prune(now_unix_ms);
        let initial = !self.initialized;
        self.initialized = true;
        let mut triggers = Vec::new();

        for session in &snapshot.sessions {
            let id = format!(
                "session:{:?}:{:?}",
                session.identity.source, session.identity.id
            );
            let waiting = matches!(
                session.waiting,
                WaitingState::UserInput | WaitingState::Approval
            );
            let revision = format!(
                "{:?}|{:?}|{:?}",
                session.waiting, session.lifecycle, session.connection
            );
            let changed = self.changed(&id, &revision, now_unix_ms);
            if !initial
                && changed
                && waiting
                && session.lifecycle != SessionLifecycle::Ended
                && preferences.waiting_enabled
            {
                triggers.push(trigger(
                    &id,
                    NotificationCategory::Waiting,
                    NotificationPriority::Waiting,
                    "Codex needs your attention",
                    session
                        .display_name
                        .as_deref()
                        .unwrap_or("A Codex session is waiting"),
                    Some(NotificationTarget::OpenCodex),
                    now_unix_ms,
                ));
            }
            if !initial
                && changed
                && !waiting
                && session.lifecycle == SessionLifecycle::Active
                && session.activity == SessionActivity::Working
                && preferences.active_enabled
            {
                triggers.push(trigger(
                    &format!("{id}:active"),
                    NotificationCategory::Active,
                    NotificationPriority::Active,
                    "Codex is working",
                    session
                        .display_name
                        .as_deref()
                        .unwrap_or("A Codex session started working"),
                    Some(NotificationTarget::OpenCodex),
                    now_unix_ms,
                ));
            }
            if !initial
                && changed
                && session.connection == ConnectionHealth::Offline
                && preferences.health_enabled
            {
                triggers.push(trigger(
                    &format!("{id}:offline"),
                    NotificationCategory::Health,
                    NotificationPriority::Failure,
                    "Codex observer is offline",
                    "Session updates may be delayed until the observer reconnects.",
                    Some(NotificationTarget::OpenCodex),
                    now_unix_ms,
                ));
            }
        }

        for turn in &snapshot.turns {
            if turn.lifecycle != TurnLifecycle::Completed
                && turn.lifecycle != TurnLifecycle::Failed
                && turn.outcome == TurnOutcome::Unknown
            {
                continue;
            }
            let id = format!("turn:{:?}:{:?}", turn.identity.source, turn.identity.id);
            let revision = format!(
                "{:?}|{:?}|{:?}",
                turn.lifecycle, turn.outcome, turn.ended_at_unix_ms
            );
            let changed = self.changed(&id, &revision, now_unix_ms);
            if initial || !changed {
                continue;
            }
            let event_at = turn.ended_at_unix_ms.unwrap_or(now_unix_ms);
            match turn.outcome {
                TurnOutcome::Failure
                    if preferences.failure_enabled
                        && now_unix_ms.saturating_sub(event_at) <= ACTIONABLE_FAILURE_MS =>
                {
                    triggers.push(trigger(
                        &id,
                        NotificationCategory::Failure,
                        NotificationPriority::Failure,
                        "Codex task failed",
                        turn.summary
                            .as_deref()
                            .unwrap_or("A Codex turn ended with a failure."),
                        Some(NotificationTarget::OpenCodex),
                        event_at,
                    ))
                }
                TurnOutcome::Success
                    if preferences.completion_enabled
                        && now_unix_ms.saturating_sub(event_at) <= RECENT_COMPLETION_MS =>
                {
                    triggers.push(trigger(
                        &id,
                        NotificationCategory::Completion,
                        NotificationPriority::Completion,
                        "Codex task completed",
                        turn.summary.as_deref().unwrap_or("A Codex turn completed."),
                        Some(NotificationTarget::OpenCodex),
                        event_at,
                    ))
                }
                TurnOutcome::Unknown
                    if turn.lifecycle == TurnLifecycle::Completed
                        && preferences.completion_enabled
                        && now_unix_ms.saturating_sub(event_at) <= RECENT_COMPLETION_MS =>
                {
                    triggers.push(trigger(
                        &id,
                        NotificationCategory::Completion,
                        NotificationPriority::Completion,
                        "Codex turn ended",
                        "The turn ended; no success or failure result was verified.",
                        Some(NotificationTarget::OpenCodex),
                        event_at,
                    ))
                }
                _ => {}
            }
        }

        for repository in &snapshot.repositories {
            let Some(sha) = repository.head_sha.as_deref() else {
                continue;
            };
            let id = format!(
                "github-head:{:?}:{:?}",
                repository.identity.source, repository.identity.id
            );
            let changed = self.changed(&id, sha, now_unix_ms);
            if !initial && changed && preferences.github_enabled {
                let target = github_url(repository.owner.as_deref(), repository.name.as_deref());
                triggers.push(trigger(
                    &id,
                    NotificationCategory::GitHub,
                    NotificationPriority::Active,
                    "Repository changed",
                    "The selected worktree is now on a different commit.",
                    target.map(|url| NotificationTarget::OpenGitHub { url }),
                    now_unix_ms,
                ));
            }
        }

        for integration in &snapshot.integrations {
            if integration.provider != IntegrationProvider::GitHub {
                continue;
            }
            let id = format!("integration:{}", integration.identity.as_str());
            let revision = format!(
                "{:?}|{:?}|{:?}",
                integration.connection, integration.last_error, integration.retry_at_unix_ms
            );
            let changed = self.changed(&id, &revision, now_unix_ms);
            if !initial
                && changed
                && integration.connection == ConnectionHealth::Offline
                && preferences.health_enabled
            {
                triggers.push(trigger(
                    &id,
                    NotificationCategory::Health,
                    NotificationPriority::Failure,
                    "GitHub is offline",
                    "GitHub status will refresh after the connection recovers.",
                    None,
                    now_unix_ms,
                ));
            }
        }

        if preferences.paused {
            triggers.clear();
        }
        self.apply_cooldown(&mut triggers, now_unix_ms);
        triggers.sort_by_key(|event| priority_rank(event.priority));
        triggers.truncate(MAX_TRIGGERS_PER_UPDATE);
        let primary = current_priority(snapshot, now_unix_ms);
        let badge_count = snapshot
            .sessions
            .iter()
            .filter(|session| {
                matches!(
                    session.waiting,
                    WaitingState::Approval | WaitingState::UserInput
                ) && session.lifecycle != SessionLifecycle::Ended
            })
            .count() as u32
            + snapshot
                .turns
                .iter()
                .filter(|turn| {
                    turn.outcome == TurnOutcome::Failure
                        && now_unix_ms.saturating_sub(turn.ended_at_unix_ms.unwrap_or(now_unix_ms))
                            <= ACTIONABLE_FAILURE_MS
                })
                .count() as u32;
        self.last_priority = primary;
        self.last_badge_count = badge_count;
        let suppressed = preferences.paused || preferences.quiet || preferences.fullscreen;
        let sound = if !suppressed && preferences.sound_enabled {
            triggers.first().and_then(|event| match event.category {
                NotificationCategory::Waiting => Some(SoundCue::Attention),
                NotificationCategory::Failure => Some(SoundCue::Failure),
                NotificationCategory::Active => Some(SoundCue::Activity),
                NotificationCategory::Completion => Some(SoundCue::Completion),
                _ => None,
            })
        } else {
            None
        };

        NotificationUpdate {
            primary,
            badge_count,
            sound,
            triggers,
            native_toast: if preferences.native_toast_enabled {
                NativeToastStatus::Unavailable
            } else {
                NativeToastStatus::Disabled
            },
            suppressed,
        }
    }

    /// Observe the typed GitHub workflow result independently from the broker's
    /// coarse repository/integration records. The first successful SHA is a
    /// silent baseline; check transitions notify only while the SHA is stable.
    pub fn process_github_snapshot(
        &mut self,
        snapshot: &crate::github::GithubSnapshot,
        now_unix_ms: u64,
        preferences: NotificationPreferences,
    ) -> NotificationUpdate {
        let mut triggers = Vec::new();
        let repository_id = snapshot
            .repository
            .worktree_id
            .as_deref()
            .or(snapshot.repository.repository_id.as_deref())
            .unwrap_or("selected-repository");
        let health_key = format!("github-health:{repository_id}");
        let had_health_baseline = self.seen.contains_key(&health_key);
        let health_revision = format!("{:?}", snapshot.health);
        let health_changed = self.changed(&health_key, &health_revision, now_unix_ms);
        if had_health_baseline
            && health_changed
            && preferences.health_enabled
            && matches!(
                snapshot.health,
                crate::github::GithubHealth::GhUnavailable
                    | crate::github::GithubHealth::Unauthenticated
                    | crate::github::GithubHealth::Offline
                    | crate::github::GithubHealth::RateLimited
                    | crate::github::GithubHealth::PermissionDenied
                    | crate::github::GithubHealth::Error
            )
        {
            let detail = match snapshot.health {
                crate::github::GithubHealth::GhUnavailable => {
                    "Install GitHub CLI to refresh pull requests and checks."
                }
                crate::github::GithubHealth::Unauthenticated => {
                    "Sign in to GitHub CLI to refresh pull requests and checks."
                }
                crate::github::GithubHealth::RateLimited => {
                    "GitHub is rate limiting refreshes; status will retry later."
                }
                crate::github::GithubHealth::PermissionDenied => {
                    "The GitHub CLI account cannot read this repository."
                }
                _ => "GitHub status will refresh after the connection recovers.",
            };
            triggers.push(trigger(
                &health_key,
                NotificationCategory::Health,
                NotificationPriority::Failure,
                "GitHub status is unavailable",
                detail,
                None,
                now_unix_ms,
            ));
        }
        if snapshot.health == crate::github::GithubHealth::Ready && !snapshot.stale {
            let sha = snapshot.repository.head_sha.as_str();
            let current_checks = check_revisions(&snapshot.checks, sha);
            let pr_url = snapshot
                .pull_request
                .as_ref()
                .filter(|pr| pr.head_matches && pr.head_sha.eq_ignore_ascii_case(sha))
                .map(|pr| pr.url.as_str());
            match self.github.get_mut(repository_id) {
                None => {
                    self.github.insert(
                        repository_id.to_owned(),
                        GithubBaseline {
                            head_sha: sha.to_owned(),
                            checks: current_checks,
                            observed_at_ms: now_unix_ms,
                        },
                    );
                }
                Some(baseline) if baseline.head_sha != sha => {
                    // SHA transitions rebaseline all checks and remain silent.
                    baseline.head_sha = sha.to_owned();
                    baseline.checks = current_checks;
                    baseline.observed_at_ms = now_unix_ms;
                }
                Some(baseline) => {
                    baseline.observed_at_ms = now_unix_ms;
                    for check in snapshot
                        .checks
                        .iter()
                        .filter(|check| check.head_sha.eq_ignore_ascii_case(sha))
                    {
                        let revision =
                            format!("{:?}|{:?}|{}", check.status, check.conclusion, check.name);
                        let previous = baseline.checks.insert(check.id.clone(), revision.clone());
                        if previous.is_some_and(|previous| previous != revision) {
                            let category = if matches!(
                                check.status,
                                crate::github::CheckState::Failed
                                    | crate::github::CheckState::Cancelled
                            ) {
                                NotificationCategory::Failure
                            } else {
                                NotificationCategory::GitHub
                            };
                            let enabled = if category == NotificationCategory::Failure {
                                preferences.failure_enabled
                            } else {
                                preferences.github_enabled
                            };
                            if enabled {
                                let target = validate_github_url(check.url.as_deref())
                                    .or_else(|| validate_github_url(pr_url))
                                    .map(|url| NotificationTarget::OpenGitHub { url });
                                triggers.push(trigger(
                                    &format!("github-check:{repository_id}:{}:{sha}", check.id),
                                    category,
                                    if category == NotificationCategory::Failure {
                                        NotificationPriority::Failure
                                    } else {
                                        NotificationPriority::Active
                                    },
                                    if category == NotificationCategory::Failure {
                                        "GitHub check failed"
                                    } else {
                                        "GitHub check updated"
                                    },
                                    &check.name,
                                    target,
                                    now_unix_ms,
                                ));
                            }
                        }
                    }
                }
            }
        }
        self.github.retain(|_, baseline| {
            now_unix_ms.saturating_sub(baseline.observed_at_ms) <= DEDUP_TTL_MS
        });
        while self.github.len() > 64 {
            if let Some(oldest) = self
                .github
                .iter()
                .min_by_key(|(_, value)| value.observed_at_ms)
                .map(|(key, _)| key.clone())
            {
                self.github.remove(&oldest);
            }
        }
        if preferences.paused {
            triggers.clear();
        }
        self.apply_cooldown(&mut triggers, now_unix_ms);
        triggers.sort_by_key(|event| priority_rank(event.priority));
        triggers.truncate(MAX_TRIGGERS_PER_UPDATE);
        let sound = if !(preferences.paused || preferences.quiet || preferences.fullscreen)
            && preferences.sound_enabled
        {
            triggers.first().and_then(|event| match event.category {
                NotificationCategory::Failure => Some(SoundCue::Failure),
                NotificationCategory::GitHub => Some(SoundCue::Activity),
                _ => None,
            })
        } else {
            None
        };
        let event_priority = if triggers
            .iter()
            .any(|event| event.priority == NotificationPriority::Failure)
        {
            NotificationPriority::Failure
        } else {
            NotificationPriority::Idle
        };
        let primary = if priority_rank(event_priority) < priority_rank(self.last_priority) {
            event_priority
        } else {
            self.last_priority
        };
        NotificationUpdate {
            primary,
            badge_count: self.last_badge_count,
            sound,
            triggers,
            native_toast: if preferences.native_toast_enabled {
                NativeToastStatus::Unavailable
            } else {
                NativeToastStatus::Disabled
            },
            suppressed: preferences.paused || preferences.quiet || preferences.fullscreen,
        }
    }

    fn changed(&mut self, key: &str, revision: &str, now_ms: u64) -> bool {
        self.ordinal = self.ordinal.wrapping_add(1);
        match self.seen.get_mut(key) {
            Some(previous) if previous.revision == revision => {
                previous.observed_at_ms = now_ms;
                previous.ordinal = self.ordinal;
                false
            }
            Some(previous) => {
                previous.revision = revision.to_owned();
                previous.observed_at_ms = now_ms;
                previous.ordinal = self.ordinal;
                true
            }
            None => {
                if self.seen.len() >= DEDUP_CAPACITY {
                    if let Some(oldest) = self
                        .seen
                        .iter()
                        .min_by_key(|(_, value)| value.ordinal)
                        .map(|(key, _)| key.clone())
                    {
                        self.seen.remove(&oldest);
                    }
                }
                self.seen.insert(
                    key.to_owned(),
                    SeenRevision {
                        revision: revision.to_owned(),
                        observed_at_ms: now_ms,
                        ordinal: self.ordinal,
                    },
                );
                true
            }
        }
    }

    fn prune(&mut self, now_ms: u64) {
        self.seen
            .retain(|_, seen| now_ms.saturating_sub(seen.observed_at_ms) <= DEDUP_TTL_MS);
        self.last_emitted
            .retain(|_, emitted| now_ms.saturating_sub(*emitted) <= DEDUP_TTL_MS);
    }

    fn apply_cooldown(&mut self, triggers: &mut Vec<NotificationTrigger>, now_ms: u64) {
        triggers.retain(|event| {
            let key = format!("{:?}:{}", event.category, event.id);
            if self
                .last_emitted
                .get(&key)
                .is_some_and(|last| now_ms.saturating_sub(*last) < SAME_TARGET_COOLDOWN_MS)
            {
                return false;
            }
            if self.last_emitted.len() >= DEDUP_CAPACITY {
                if let Some(oldest) = self
                    .last_emitted
                    .iter()
                    .min_by_key(|(_, emitted)| *emitted)
                    .map(|(key, _)| key.clone())
                {
                    self.last_emitted.remove(&oldest);
                }
            }
            self.last_emitted.insert(key, now_ms);
            true
        });
    }
}

fn trigger(
    id: &str,
    category: NotificationCategory,
    priority: NotificationPriority,
    title: &str,
    detail: &str,
    target: Option<NotificationTarget>,
    occurred_at_unix_ms: u64,
) -> NotificationTrigger {
    NotificationTrigger {
        id: id.to_owned(),
        category,
        priority,
        title: title.to_owned(),
        detail: detail.to_owned(),
        target,
        occurred_at_unix_ms,
    }
}

fn priority_rank(priority: NotificationPriority) -> u8 {
    match priority {
        NotificationPriority::Waiting => 0,
        NotificationPriority::Failure => 1,
        NotificationPriority::Active => 2,
        NotificationPriority::Completion => 3,
        NotificationPriority::Idle => 4,
    }
}

fn current_priority(snapshot: &BrokerSnapshot, now_ms: u64) -> NotificationPriority {
    if snapshot.sessions.iter().any(|session| {
        matches!(
            session.waiting,
            WaitingState::Approval | WaitingState::UserInput
        ) && session.lifecycle != SessionLifecycle::Ended
    }) {
        NotificationPriority::Waiting
    } else if snapshot.turns.iter().any(|turn| {
        turn.outcome == TurnOutcome::Failure
            && now_ms.saturating_sub(turn.ended_at_unix_ms.unwrap_or(now_ms))
                <= ACTIONABLE_FAILURE_MS
    }) || snapshot
        .sessions
        .iter()
        .any(|session| session.connection == ConnectionHealth::Offline)
    {
        NotificationPriority::Failure
    } else if snapshot.sessions.iter().any(|session| {
        session.lifecycle == SessionLifecycle::Active
            && session.activity == SessionActivity::Working
    }) {
        NotificationPriority::Active
    } else if snapshot.turns.iter().any(|turn| {
        (turn.outcome == TurnOutcome::Success
            || (turn.lifecycle == TurnLifecycle::Completed && turn.outcome == TurnOutcome::Unknown))
            && now_ms.saturating_sub(turn.ended_at_unix_ms.unwrap_or(now_ms))
                <= RECENT_COMPLETION_MS
    }) {
        NotificationPriority::Completion
    } else {
        NotificationPriority::Idle
    }
}

fn github_url(owner: Option<&str>, name: Option<&str>) -> Option<String> {
    fn component(value: &str) -> bool {
        !value.is_empty()
            && value.len() <= 100
            && value
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
            && value != "."
            && value != ".."
    }
    let (owner, name) = (owner?, name?);
    (component(owner) && component(name)).then(|| format!("https://github.com/{owner}/{name}"))
}

fn validate_github_url(url: Option<&str>) -> Option<String> {
    let url = url?;
    let without_scheme = url.strip_prefix("https://github.com/")?;
    let parts = without_scheme.split('/').collect::<Vec<_>>();
    let owner = *parts.first()?;
    let repo = *parts.get(1)?;
    github_url(Some(owner), Some(repo))?;
    let valid_detail = match parts.as_slice() {
        [_, _] => true,
        [_, _, "pull", number] | [_, _, "issues", number] => {
            !number.is_empty() && number.bytes().all(|ch| ch.is_ascii_digit())
        }
        [_, _, "actions", "runs", number] => {
            !number.is_empty() && number.bytes().all(|ch| ch.is_ascii_digit())
        }
        _ => false,
    };
    if !valid_detail {
        return None;
    }
    Some(url.to_owned())
}

fn check_revisions(
    checks: &[crate::github::CheckSnapshot],
    head_sha: &str,
) -> HashMap<String, String> {
    checks
        .iter()
        .filter(|check| check.head_sha.eq_ignore_ascii_case(head_sha))
        .map(|check| {
            (
                check.id.clone(),
                format!("{:?}|{:?}|{}", check.status, check.conclusion, check.name),
            )
        })
        .collect()
}

/// XML-escapes untrusted user/repository text for a future toast body. No
/// Windows notification is shown by this module; escaping is tested now.
pub fn toast_xml(title: &str, body: &str) -> String {
    format!("<toast><visual><binding template=\"ToastGeneric\"><text>{}</text><text>{}</text></binding></visual></toast>", xml_escape(title), xml_escape(body))
}

fn xml_escape(value: &str) -> String {
    value
        .chars()
        .filter(|ch| {
            let code = *ch as u32;
            matches!(code, 0x9 | 0xA | 0xD)
                || (0x20..=0xD7FF).contains(&code)
                || (0xE000..=0xFFFD).contains(&code)
                || (0x10000..=0x10FFFF).contains(&code)
        })
        .take(2048)
        .map(|ch| match ch {
            '&' => "&amp;".to_owned(),
            '<' => "&lt;".to_owned(),
            '>' => "&gt;".to_owned(),
            '\"' => "&quot;".to_owned(),
            '\'' => "&apos;".to_owned(),
            _ => ch.to_string(),
        })
        .collect()
}

/// Activation data is accepted only for explicitly supported targets. Since
/// no toast adapter is registered, there are no live activation tokens. A
/// token retained across a session restart should use this stale explanation.
pub fn stale_activation_message() -> &'static str {
    "This notification is out of date. Open Codex or GitHub to see the current state."
}

/// Best-effort foreground fullscreen detection for quiet policy. A positive
/// result means the foreground window covers its nearest monitor within two
/// pixels; it does not classify the application or guarantee exclusive mode.
#[cfg(windows)]
pub fn foreground_window_is_fullscreen() -> bool {
    use windows::Win32::Graphics::Gdi::{
        GetMonitorInfoW, MonitorFromWindow, MONITORINFO, MONITOR_DEFAULTTONEAREST,
    };
    use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowRect};

    let window = unsafe { GetForegroundWindow() };
    if window.is_invalid() {
        return false;
    }
    let monitor = unsafe { MonitorFromWindow(window, MONITOR_DEFAULTTONEAREST) };
    if monitor.0.is_null() {
        return false;
    }
    let mut monitor_info = MONITORINFO {
        cbSize: std::mem::size_of::<MONITORINFO>() as u32,
        ..Default::default()
    };
    if !unsafe { GetMonitorInfoW(monitor, &mut monitor_info) }.as_bool() {
        return false;
    }
    let mut window_rect = windows::Win32::Foundation::RECT::default();
    if unsafe { GetWindowRect(window, &mut window_rect) }.is_err() {
        return false;
    }
    const TOLERANCE: i32 = 2;
    window_rect.left <= monitor_info.rcMonitor.left + TOLERANCE
        && window_rect.top <= monitor_info.rcMonitor.top + TOLERANCE
        && window_rect.right >= monitor_info.rcMonitor.right - TOLERANCE
        && window_rect.bottom >= monitor_info.rcMonitor.bottom - TOLERANCE
}

#[cfg(not(windows))]
pub fn foreground_window_is_fullscreen() -> bool {
    false
}

/// Submit a toast only after an explicit user opt-in. `Show` returning `Ok`
/// means Windows accepted the API call; it does not confirm visible delivery
/// or activation. The caller must keep activation and delivery marked pending.
#[cfg(windows)]
pub fn submit_native_toast(title: &str, body: &str, opt_in: bool) -> NativeToastStatus {
    if !opt_in {
        return NativeToastStatus::Disabled;
    }
    use windows::core::HSTRING;
    use windows::Data::Xml::Dom::XmlDocument;
    use windows::UI::Notifications::{ToastNotification, ToastNotificationManager};

    let xml = HSTRING::from(toast_xml(title, body));
    let document =
        match XmlDocument::new().and_then(|document| document.LoadXml(&xml).map(|_| document)) {
            Ok(document) => document,
            Err(_) => return NativeToastStatus::Unavailable,
        };
    let notification = match ToastNotification::CreateToastNotification(&document) {
        Ok(notification) => notification,
        Err(_) => return NativeToastStatus::Unavailable,
    };
    let app_id = HSTRING::from(APP_AUMID);
    let notifier = match ToastNotificationManager::CreateToastNotifierWithId(&app_id) {
        Ok(notifier) => notifier,
        Err(_) => return NativeToastStatus::Unavailable,
    };
    match notifier.Show(&notification) {
        Ok(()) => NativeToastStatus::SubmittedUnverified,
        Err(_) => NativeToastStatus::Unavailable,
    }
}

#[cfg(not(windows))]
pub fn submit_native_toast(_title: &str, _body: &str, opt_in: bool) -> NativeToastStatus {
    if opt_in {
        NativeToastStatus::Unavailable
    } else {
        NativeToastStatus::Disabled
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::broker::types::{
        OpaqueId, Repository, SchemaVersion, Session, SourceKind, SourceScopedId, Turn,
    };

    fn empty_snapshot() -> BrokerSnapshot {
        BrokerSnapshot {
            schema_version: SchemaVersion::current(),
            sequence: 1,
            sessions: vec![],
            turns: vec![],
            tool_items: vec![],
            pending_requests: vec![],
            repositories: vec![],
            integrations: vec![],
        }
    }

    fn session(waiting: WaitingState) -> Session {
        Session {
            schema_version: SchemaVersion::current(),
            identity: SourceScopedId::new(
                SourceKind::CodexCliObserver,
                OpaqueId::new("session-1").unwrap(),
            ),
            thread_id: None,
            agent_id: None,
            parent_agent_id: None,
            repository_id: None,
            worktree_id: None,
            display_name: Some("Build".into()),
            model: None,
            lifecycle: SessionLifecycle::Active,
            activity: SessionActivity::Working,
            waiting,
            connection: ConnectionHealth::Healthy,
            started_at_unix_ms: Some(1),
            last_seen_at_unix_ms: Some(1),
        }
    }

    #[test]
    fn first_snapshot_is_silent_then_waiting_transition_deduplicates() {
        let mut service = NotificationService::default();
        let mut snapshot = empty_snapshot();
        snapshot.sessions.push(session(WaitingState::NotWaiting));
        assert!(service
            .process_snapshot(
                &snapshot,
                100,
                NotificationPreferences {
                    quiet: false,
                    ..Default::default()
                }
            )
            .triggers
            .is_empty());
        snapshot.sessions[0].waiting = WaitingState::Approval;
        let prefs = NotificationPreferences {
            quiet: false,
            ..Default::default()
        };
        let update = service.process_snapshot(&snapshot, 200, prefs);
        assert_eq!(update.primary, NotificationPriority::Waiting);
        assert_eq!(update.triggers.len(), 1);
        assert_eq!(update.sound, Some(SoundCue::Attention));
        assert!(service
            .process_snapshot(&snapshot, 300, prefs)
            .triggers
            .is_empty());
    }

    #[test]
    fn rapid_repeat_for_same_waiting_target_is_rate_limited() {
        let mut service = NotificationService::default();
        let mut snapshot = empty_snapshot();
        snapshot.sessions.push(session(WaitingState::NotWaiting));
        let prefs = NotificationPreferences {
            quiet: false,
            ..Default::default()
        };
        service.process_snapshot(&snapshot, 100, prefs);
        snapshot.sessions[0].waiting = WaitingState::Approval;
        assert_eq!(
            service
                .process_snapshot(&snapshot, 200, prefs)
                .triggers
                .len(),
            1
        );
        snapshot.sessions[0].waiting = WaitingState::NotWaiting;
        service.process_snapshot(&snapshot, 300, prefs);
        snapshot.sessions[0].waiting = WaitingState::UserInput;
        assert!(service
            .process_snapshot(&snapshot, 500, prefs)
            .triggers
            .is_empty());
        snapshot.sessions[0].waiting = WaitingState::NotWaiting;
        service.process_snapshot(&snapshot, 600, prefs);
        snapshot.sessions[0].waiting = WaitingState::Approval;
        assert_eq!(
            service
                .process_snapshot(&snapshot, 16_000, prefs)
                .triggers
                .len(),
            1
        );
    }

    #[test]
    fn github_sha_is_an_independent_transition_and_url_is_validated() {
        let mut service = NotificationService::default();
        let mut snapshot = empty_snapshot();
        snapshot.repositories.push(Repository {
            schema_version: SchemaVersion::current(),
            identity: SourceScopedId::new(SourceKind::GitHub, OpaqueId::new("repo-id").unwrap()),
            host: Some("github.com".into()),
            owner: Some("org".into()),
            name: Some("repo".into()),
            worktree_id: Some(OpaqueId::new("wt").unwrap()),
            branch: Some("main".into()),
            head_sha: Some("abc".into()),
            observed_at_unix_ms: Some(1),
        });
        let quiet = NotificationPreferences::default();
        service.process_snapshot(&snapshot, 100, quiet);
        snapshot.repositories[0].head_sha = Some("def".into());
        let update = service.process_snapshot(&snapshot, 200, quiet);
        assert_eq!(update.triggers.len(), 1);
        assert_eq!(update.triggers[0].category, NotificationCategory::GitHub);
        assert_eq!(
            update.triggers[0].target,
            Some(NotificationTarget::OpenGitHub {
                url: "https://github.com/org/repo".into()
            })
        );
        assert_eq!(github_url(Some("bad/owner"), Some("repo")), None);
    }

    #[test]
    fn github_checks_seed_silently_and_only_notify_same_sha_transitions() {
        use crate::github::{
            CheckKind, CheckSnapshot, CheckState, GithubHealth, GithubSnapshot, PullRequestSnapshot,
        };
        use crate::repositories::RepositoryIdentity;

        let repository = RepositoryIdentity {
            canonical_path: "C:/private/repo".into(),
            common_dir: "C:/private/repo/.git".into(),
            worktree_dir: "C:/private/repo".into(),
            branch: "main".into(),
            head_sha: "sha-a".into(),
            remote_name: "origin".into(),
            remote_url: "https://github.com/org/repo".into(),
            owner: "org".into(),
            name: "repo".into(),
            repository_id: Some("repository-1".into()),
            worktree_id: Some("worktree-1".into()),
            github_remote_repositories: vec![],
        };
        let pr = PullRequestSnapshot {
            number: 7,
            title: "title".into(),
            state: "OPEN".into(),
            is_draft: false,
            url: "https://github.com/org/repo/pull/7".into(),
            head_owner: "org".into(),
            head_repo: "repo".into(),
            head_ref: "main".into(),
            head_sha: "sha-a".into(),
            base_ref: "main".into(),
            review_decision: None,
            head_matches: true,
        };
        let mut snapshot = GithubSnapshot {
            repository,
            pull_request: Some(pr),
            checks: vec![CheckSnapshot {
                id: "42".into(),
                name: "build".into(),
                kind: CheckKind::CheckRun,
                status: CheckState::InProgress,
                conclusion: None,
                url: Some("https://github.com/org/repo/actions/runs/42".into()),
                head_sha: "sha-a".into(),
            }],
            health: GithubHealth::Ready,
            fetched_at_unix_ms: Some(100),
            stale: false,
            error: None,
        };
        let prefs = NotificationPreferences {
            quiet: false,
            ..Default::default()
        };
        let mut service = NotificationService::default();
        assert!(service
            .process_github_snapshot(&snapshot, 100, prefs)
            .triggers
            .is_empty());

        snapshot.checks[0].status = CheckState::Failed;
        snapshot.checks[0].conclusion = Some("failure".into());
        let changed = service.process_github_snapshot(&snapshot, 200, prefs);
        assert_eq!(changed.triggers.len(), 1);
        assert_eq!(changed.triggers[0].priority, NotificationPriority::Failure);
        assert_eq!(
            changed.triggers[0].target,
            Some(NotificationTarget::OpenGitHub {
                url: "https://github.com/org/repo/actions/runs/42".into()
            })
        );
        assert!(service
            .process_github_snapshot(&snapshot, 300, prefs)
            .triggers
            .is_empty());

        snapshot.pull_request.as_mut().unwrap().head_sha = "sha-b".into();
        snapshot.checks[0].head_sha = "sha-b".into();
        snapshot.checks[0].status = CheckState::InProgress;
        snapshot.checks[0].conclusion = None;
        assert!(service
            .process_github_snapshot(&snapshot, 400, prefs)
            .triggers
            .is_empty());

        // Check transitions are tied to the selected commit, not the presence
        // of an open PR. This covers commits on branches without a PR.
        snapshot.repository.head_sha = "sha-b".into();
        snapshot.pull_request = None;
        snapshot.checks[0].head_sha = "sha-b".into();
        snapshot.checks[0].status = CheckState::InProgress;
        snapshot.checks[0].conclusion = None;
        let mut without_pr = NotificationService::default();
        assert!(without_pr
            .process_github_snapshot(&snapshot, 500, prefs)
            .triggers
            .is_empty());
        snapshot.checks[0].status = CheckState::Failed;
        snapshot.checks[0].conclusion = Some("failure".into());
        let changed = without_pr.process_github_snapshot(&snapshot, 600, prefs);
        assert_eq!(changed.triggers.len(), 1);
        assert_eq!(changed.triggers[0].priority, NotificationPriority::Failure);
        assert_eq!(
            changed.triggers[0].target,
            Some(NotificationTarget::OpenGitHub {
                url: "https://github.com/org/repo/actions/runs/42".into()
            })
        );
    }

    #[test]
    fn failures_rank_above_active_and_completion_and_quiet_suppresses_sound() {
        let mut service = NotificationService::default();
        let mut snapshot = empty_snapshot();
        let id = |value: &str| {
            SourceScopedId::new(SourceKind::CodexCliObserver, OpaqueId::new(value).unwrap())
        };
        snapshot.turns = ["failure", "success"]
            .into_iter()
            .map(|name| Turn {
                schema_version: SchemaVersion::current(),
                identity: id(name),
                session_id: id("session"),
                thread_id: None,
                agent_id: None,
                lifecycle: TurnLifecycle::Completed,
                outcome: if name == "failure" {
                    TurnOutcome::Failure
                } else {
                    TurnOutcome::Success
                },
                started_at_unix_ms: Some(1),
                ended_at_unix_ms: Some(2),
                summary: None,
                plan: None,
            })
            .collect();
        service.process_snapshot(&snapshot, 100, NotificationPreferences::default());
        let mut changed = snapshot.clone();
        changed.turns[0].ended_at_unix_ms = Some(3);
        changed.turns[1].ended_at_unix_ms = Some(3);
        let update = service.process_snapshot(&changed, 200, NotificationPreferences::default());
        assert_eq!(update.triggers[0].priority, NotificationPriority::Failure);
        assert_eq!(update.sound, None);
        assert!(update.suppressed);
    }

    #[test]
    fn native_toasts_report_unavailable_even_when_opted_in_and_xml_escapes_text() {
        let mut service = NotificationService::default();
        let update = service.process_snapshot(
            &empty_snapshot(),
            1,
            NotificationPreferences {
                native_toast_enabled: true,
                ..Default::default()
            },
        );
        assert_eq!(update.native_toast, NativeToastStatus::Unavailable);
        assert_eq!(toast_xml("<hello & \"world\"", "a > b"), "<toast><visual><binding template=\"ToastGeneric\"><text>&lt;hello &amp; &quot;world&quot;</text><text>a &gt; b</text></binding></visual></toast>");
        assert!(!toast_xml("title\u{0}safe", "body").contains('\0'));
        assert!(stale_activation_message().contains("out of date"));
        assert_eq!(APP_AUMID, "YashwanthDevelops.AntiScrollingNotch");
        assert_eq!(
            submit_native_toast("title", "body", false),
            NativeToastStatus::Disabled
        );
    }

    #[test]
    fn completed_unknown_outcome_emits_neutral_end_state_without_success_claim() {
        let mut service = NotificationService::default();
        let mut snapshot = empty_snapshot();
        service.process_snapshot(&snapshot, 100, NotificationPreferences::default());
        snapshot.turns.push(Turn {
            schema_version: SchemaVersion::current(),
            identity: SourceScopedId::new(
                SourceKind::CodexCliObserver,
                OpaqueId::new("turn-unknown").unwrap(),
            ),
            session_id: SourceScopedId::new(
                SourceKind::CodexCliObserver,
                OpaqueId::new("session").unwrap(),
            ),
            thread_id: None,
            agent_id: None,
            lifecycle: TurnLifecycle::Completed,
            outcome: TurnOutcome::Unknown,
            started_at_unix_ms: Some(10),
            ended_at_unix_ms: Some(150),
            summary: None,
            plan: None,
        });
        let update = service.process_snapshot(&snapshot, 200, NotificationPreferences::default());
        assert_eq!(update.primary, NotificationPriority::Completion);
        assert_eq!(update.triggers.len(), 1);
        assert_eq!(update.triggers[0].title, "Codex turn ended");
        assert!(update.triggers[0].detail.contains("no success or failure"));
    }

    #[test]
    fn dedup_storage_is_bounded_and_expires() {
        let mut service = NotificationService::default();
        for index in 0..(DEDUP_CAPACITY + 10) {
            service.changed(&format!("key-{index}"), "revision", 1);
        }
        assert_eq!(service.seen.len(), DEDUP_CAPACITY);
        service.prune(DEDUP_TTL_MS + 2);
        assert!(service.seen.is_empty());
    }
}
