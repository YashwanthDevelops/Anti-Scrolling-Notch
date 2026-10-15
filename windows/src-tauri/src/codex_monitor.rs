//! Bounded, identity-aware Codex CLI lifecycle normalization.
//!
//! The adapter can represent the IDs captured by CLI 0.157.1, but the current
//! pipe message does not authenticate which CLI version invoked the hook or
//! prove that the supported hook registration is active. Production broker
//! updates therefore remain gated off until a trusted runtime evidence path
//! supplies all required facts.

use std::collections::BTreeSet;
use std::sync::Mutex;

use codex_hook_contract::{CodexHookEvent, CodexHookLifecycleObservation, CodexHookObservation};
use serde::Serialize;
use tauri::Emitter;

use crate::broker::reducer::BrokerUpdate;
use crate::broker::service::BrokerService;
use crate::broker::stream::ApplyReceipt;
use crate::broker::types::{
    ConnectionHealth, OpaqueId, SchemaVersion, Session, SessionActivity, SessionLifecycle,
    SourceKind, SourceScopedId, ToolItemState, Turn, TurnLifecycle, TurnOutcome, WaitingState,
};
use crate::storage::{HistoryStore, MAX_HISTORY_EVENTS, MAX_HISTORY_READ_PAGE};

pub const SUPPORTED_CLI_VERSION: &str = "0.157.1";
const MAX_TRACKED_TERMINAL_SESSIONS: usize = 256;
const MAX_MONITORED_SESSIONS: usize = 512;
const MAX_MONITORED_TURNS: usize = 2_048;

#[derive(Clone, Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CodexMonitorStatus {
    pub schema_version: u8,
    pub capability: &'static str,
    pub supported_cli_version: &'static str,
    pub runtime_version: Option<String>,
    pub configured_hook_events: Vec<String>,
    pub hook_registration_verified: bool,
    pub relay_ready: bool,
    pub cross_session_identity_verified: bool,
    pub observer_state: ObserverState,
    pub last_event_at_unix_ms: Option<u64>,
    pub last_error: Option<String>,
    pub history_available: bool,
    pub broker_state_recovered: bool,
    pub recovery_partial: bool,
    pub recovered_event_count: usize,
    pub recovery_scope: &'static str,
    pub capacity_reached: bool,
    pub pipeline_available: bool,
    pub production_enabled: bool,
    pub disabled_reason: Option<&'static str>,
}

#[derive(Clone, Copy, Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ObserverState {
    Starting,
    Listening,
    Degraded,
    Offline,
}

#[derive(Clone, Debug, Default)]
struct RuntimeEvidence {
    cli_version: Option<String>,
    cli_supported: bool,
    configured_hook_events: Vec<String>,
    hook_registration_verified: bool,
    relay_ready: bool,
    cross_session_identity_verified: bool,
    observer_ready: bool,
    history_ready: bool,
}

impl RuntimeEvidence {
    fn permits_production(&self) -> bool {
        let expected_cli_version = format!("codex-cli {SUPPORTED_CLI_VERSION}");
        self.cli_supported
            && self.cli_version.as_deref() == Some(expected_cli_version.as_str())
            && self.hook_registration_verified
            && self.relay_ready
            && self.cross_session_identity_verified
            && self.observer_ready
            && self.history_ready
    }
}

#[derive(Debug)]
struct MonitorState {
    evidence: RuntimeEvidence,
    observer_state: ObserverState,
    last_event_at_unix_ms: Option<u64>,
    last_error: Option<String>,
    terminal_sessions: BTreeSet<String>,
    broker_state_recovered: bool,
    recovery_partial: bool,
    recovered_event_count: usize,
    capacity_reached: bool,
}

impl Default for MonitorState {
    fn default() -> Self {
        Self {
            evidence: RuntimeEvidence::default(),
            observer_state: ObserverState::Starting,
            last_event_at_unix_ms: None,
            last_error: None,
            terminal_sessions: BTreeSet::new(),
            broker_state_recovered: false,
            recovery_partial: false,
            recovered_event_count: 0,
            capacity_reached: false,
        }
    }
}

/// One process-wide status and production gate for the Codex CLI observer.
#[derive(Default)]
pub struct CodexMonitor {
    state: Mutex<MonitorState>,
    apply_lock: Mutex<()>,
}

impl CodexMonitor {
    fn record_failure(&self, error: String) -> String {
        let safe = redact_error(&error);
        self.state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .last_error = Some(safe.clone());
        safe
    }

    pub fn status(&self) -> CodexMonitorStatus {
        let state = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let enabled = state.evidence.permits_production();
        CodexMonitorStatus {
            schema_version: 2,
            capability: if enabled {
                "supported"
            } else {
                "experimental_disabled"
            },
            supported_cli_version: SUPPORTED_CLI_VERSION,
            runtime_version: state.evidence.cli_version.clone(),
            configured_hook_events: state.evidence.configured_hook_events.clone(),
            hook_registration_verified: state.evidence.hook_registration_verified,
            relay_ready: state.evidence.relay_ready,
            cross_session_identity_verified: state.evidence.cross_session_identity_verified,
            observer_state: state.observer_state,
            last_event_at_unix_ms: state.last_event_at_unix_ms,
            last_error: state.last_error.clone(),
            history_available: state.evidence.history_ready,
            broker_state_recovered: state.broker_state_recovered,
            recovery_partial: state.recovery_partial,
            recovered_event_count: state.recovered_event_count,
            recovery_scope: "retained_normalized_codex_history",
            capacity_reached: state.capacity_reached,
            pipeline_available: true,
            production_enabled: enabled,
            disabled_reason: (!enabled).then_some(
                "Trusted hook invocation context, verified 0.157.1 registration, cross-session identity evidence, observer readiness, and history storage are required.",
            ),
        }
    }

    /// Rebuild the retained Codex CLI view from bounded normalized history.
    /// Pending requests and non-Codex state are intentionally excluded; all
    /// restored live states are marked unknown until new hook events arrive.
    pub fn recover_history(
        &self,
        history: &HistoryStore,
        broker: &BrokerService,
    ) -> Result<HistoryRecoveryReport, String> {
        let _apply_guard = self
            .apply_lock
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let mut rows = Vec::new();
        let mut cursor = 0_i64;
        let mut truncated = false;
        while rows.len() < MAX_HISTORY_EVENTS {
            let page = history
                .records_after(cursor, MAX_HISTORY_READ_PAGE)
                .map_err(|error| self.record_failure(error.to_string()))?;
            if page.is_empty() {
                break;
            }
            cursor = page.last().map(|record| record.id).unwrap_or(cursor);
            rows.extend(page);
        }
        if rows.len() >= MAX_HISTORY_EVENTS {
            let beyond = history
                .records_after(cursor, 1)
                .map_err(|error| self.record_failure(error.to_string()))?;
            truncated = !beyond.is_empty();
        }

        let mut updates = Vec::new();
        let mut recovered_sessions = BTreeSet::new();
        let mut recovered_turns = BTreeSet::new();
        let mut skipped = 0usize;
        let mut previous_source_sequence = None;
        let mut replay_timestamp = 0u64;
        for record in rows.iter().filter(|record| {
            record.record_kind == "broker_update" && record.source == "codex_cli_observer"
        }) {
            let Ok(event) = serde_json::from_str::<crate::broker::types::EventEnvelope<BrokerUpdate>>(
                &record.record_json,
            ) else {
                skipped += 1;
                continue;
            };
            let sequence = event.sequence.get();
            if event.source != SourceKind::CodexCliObserver
                || record.source_sequence != Some(sequence)
                || previous_source_sequence.is_some_and(|previous| sequence <= previous)
            {
                skipped += 1;
                continue;
            }
            previous_source_sequence = Some(sequence);
            replay_timestamp = replay_timestamp.max(event.observed_at_unix_ms);
            let Some(update) = restart_safe_update(event.payload) else {
                continue;
            };
            if !within_recovery_limits(&update, &mut recovered_sessions, &mut recovered_turns) {
                skipped += 1;
                continue;
            }
            updates.push(update);
        }

        let replay = broker
            .apply_batch(updates, replay_timestamp)
            .map_err(|error| self.record_failure(error.to_string()))?;
        broker
            .advance_after_recovery(previous_source_sequence.unwrap_or(0))
            .map_err(|error| self.record_failure(error.to_string()))?;
        let stale_timestamp = replay_timestamp.saturating_add(1);
        let stale = broker
            .apply_batch_with(stale_timestamp, |snapshot| {
                let mut updates = Vec::new();
                for session in &snapshot.sessions {
                    if session.identity.source != SourceKind::CodexCliObserver {
                        continue;
                    }
                    updates.push(BrokerUpdate::PatchSessionState(
                        crate::broker::reducer::SessionStatePatch {
                            identity: session.identity.clone(),
                            lifecycle: (session.lifecycle != SessionLifecycle::Ended)
                                .then_some(SessionLifecycle::Unknown),
                            activity: Some(SessionActivity::Unknown),
                            waiting: Some(WaitingState::Unknown),
                            connection: Some(ConnectionHealth::Unknown),
                        },
                    ));
                }
                for turn in &snapshot.turns {
                    if turn.identity.source != SourceKind::CodexCliObserver
                        || turn.lifecycle != TurnLifecycle::Active
                    {
                        continue;
                    }
                    let mut turn = turn.clone();
                    turn.lifecycle = TurnLifecycle::Unknown;
                    turn.outcome = TurnOutcome::Unknown;
                    updates.push(BrokerUpdate::UpsertTurn(turn));
                }
                for tool in &snapshot.tool_items {
                    if tool.identity.source != SourceKind::CodexCliObserver
                        || tool.state != ToolItemState::Active
                    {
                        continue;
                    }
                    let mut tool = tool.clone();
                    tool.state = ToolItemState::Unknown;
                    updates.push(BrokerUpdate::UpsertToolItem(tool));
                }
                updates
            })
            .map_err(|error| self.record_failure(error.to_string()))?;
        let _ = stale;

        let report = HistoryRecoveryReport {
            scanned_rows: rows.len(),
            recovered_events: replay.events.len(),
            skipped_rows: skipped,
            truncated,
            last_source_sequence: previous_source_sequence,
        };
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        state.evidence.history_ready = true;
        state.broker_state_recovered = true;
        state.recovery_partial = truncated || skipped > 0;
        state.recovered_event_count = report.recovered_events;
        state.capacity_reached = recovered_sessions.len() >= MAX_MONITORED_SESSIONS
            || recovered_turns.len() >= MAX_MONITORED_TURNS;
        Ok(report)
    }

    /// Update pipe health. Listening alone never opens the production gate.
    pub fn set_observer_state(&self, observer_state: ObserverState, error: Option<String>) {
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        state.observer_state = observer_state;
        state.evidence.observer_ready = observer_state == ObserverState::Listening;
        state.last_error = error.map(|value| redact_error(&value));
    }

    /// Record the read-only hook status snapshot shown to the user. Registration
    /// is verified only when all four captured event types point to the owned
    /// relay and the relay executable exists. This does not prove `/hooks`
    /// trust or cross-session identity; both remain explicitly unverified.
    pub fn set_hook_readiness(
        &self,
        cli_version: Option<String>,
        cli_supported: bool,
        configured_events: Vec<String>,
        relay_ready: bool,
    ) {
        let required = CodexHookEvent::VERIFIED
            .iter()
            .map(|event| event.as_str())
            .collect::<BTreeSet<_>>();
        let configured = configured_events
            .iter()
            .map(String::as_str)
            .collect::<BTreeSet<_>>();
        let registration_verified = required.is_subset(&configured) && relay_ready;
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let expected_cli_version = format!("codex-cli {SUPPORTED_CLI_VERSION}");
        state.evidence.cli_supported =
            cli_supported && cli_version.as_deref() == Some(expected_cli_version.as_str());
        state.evidence.cli_version = cli_version;
        state.evidence.configured_hook_events = configured_events;
        state.evidence.hook_registration_verified = registration_verified;
        state.evidence.relay_ready = relay_ready;
        state.evidence.cross_session_identity_verified = false;
    }

    /// Consume one already-sanitized lifecycle message. The no-evidence path
    /// records only the prior anonymous v1 observation. The gated path applies
    /// normalized broker changes and persists their actual local sequence.
    pub fn apply(
        &self,
        app: &tauri::AppHandle,
        shared: &crate::Shared,
        observation: CodexHookLifecycleObservation,
        observed_at_unix_ms: u64,
    ) -> Result<ApplyReceipt, String> {
        let _apply_guard = self
            .apply_lock
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let history_available = shared
            .history
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .is_some();
        let enabled = {
            let mut state = self
                .state
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            state.last_event_at_unix_ms = Some(observed_at_unix_ms);
            state.last_error = None;
            state.evidence.history_ready = history_available;
            state.evidence.permits_production()
        };

        if !enabled {
            let anonymous = CodexHookObservation::new(observation.event);
            let mut history = shared
                .history
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            if let Some(history) = history.as_mut() {
                if let Err(error) = history.record_hook_observation(anonymous, observed_at_unix_ms)
                {
                    return Err(self.record_failure(error.to_string()));
                }
            } else {
                return Err(self.record_failure("Local history storage is unavailable.".into()));
            }
            return Ok(ApplyReceipt {
                outcome: crate::broker::reducer::ApplyOutcome::Unchanged,
                sequence: 0,
                deduplicated: false,
            });
        }

        let terminal_sessions = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .terminal_sessions
            .clone();
        let session_id = observation
            .session_id
            .as_deref()
            .and_then(|value| OpaqueId::new(value.to_owned()).ok())
            .map(|id| SourceScopedId::new(SourceKind::CodexCliObserver, id));
        let turn_id = observation
            .turn_id
            .as_deref()
            .and_then(|value| OpaqueId::new(value.to_owned()).ok())
            .map(|id| SourceScopedId::new(SourceKind::CodexCliObserver, id));
        let batch = shared
            .broker
            .apply_batch_with(observed_at_unix_ms, |snapshot| {
                let existing_session = session_id.as_ref().and_then(|id| {
                    snapshot
                        .sessions
                        .iter()
                        .find(|session| &session.identity == id)
                });
                let existing_turn = turn_id
                    .as_ref()
                    .and_then(|id| snapshot.turns.iter().find(|turn| &turn.identity == id));
                let new_session = session_id.is_some() && existing_session.is_none();
                let new_turn = turn_id.is_some() && existing_turn.is_none();
                let turn_capacity_reached = new_turn && snapshot.turns.len() >= MAX_MONITORED_TURNS;
                if (new_session && snapshot.sessions.len() >= MAX_MONITORED_SESSIONS)
                    || (turn_capacity_reached
                        && observation.event == CodexHookEvent::UserPromptSubmit)
                {
                    self.state
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner)
                        .capacity_reached = true;
                    return Vec::new();
                }
                let has_other_active_turn = snapshot.turns.iter().any(|turn| {
                    session_id.as_ref() == Some(&turn.session_id)
                        && turn.lifecycle == TurnLifecycle::Active
                        && turn_id.as_ref() != Some(&turn.identity)
                });
                let mut updates = normalize_observation(
                    &observation,
                    observed_at_unix_ms,
                    &terminal_sessions,
                    existing_session,
                    existing_turn,
                    has_other_active_turn,
                );
                if turn_capacity_reached {
                    self.state
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner)
                        .capacity_reached = true;
                    updates.retain(|update| !matches!(update, BrokerUpdate::UpsertTurn(_)));
                }
                updates
            })
            .map_err(|error| self.record_failure(error.to_string()))?;
        let latest = batch.receipts.last().copied().unwrap_or(ApplyReceipt {
            outcome: crate::broker::reducer::ApplyOutcome::Unchanged,
            sequence: 0,
            deduplicated: false,
        });
        if observation.event == CodexHookEvent::SessionEnd {
            if let Some(session_id) = observation.session_id.as_deref() {
                let mut state = self
                    .state
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner);
                if state.terminal_sessions.len() >= MAX_TRACKED_TERMINAL_SESSIONS {
                    if let Some(oldest) = state.terminal_sessions.iter().next().cloned() {
                        state.terminal_sessions.remove(&oldest);
                    }
                }
                state.terminal_sessions.insert(session_id.to_owned());
            }
        }
        let history_result = if !batch.events.is_empty() {
            let mut history = shared
                .history
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            if let Some(history) = history.as_mut() {
                history
                    .record_broker_events(&batch.events, false)
                    .map_err(|error| error.to_string())
            } else {
                Err("Local history storage is unavailable.".into())
            }
        } else {
            Ok(Vec::new())
        };
        if let Some(notification) = batch.notification {
            if let Err(error) = app.emit("broker-sync", notification) {
                return Err(self.record_failure(format!(
                    "could not notify the frontend broker view store: {error}"
                )));
            }
        }
        if let Err(error) = history_result {
            return Err(self.record_failure(error));
        }
        if !batch.events.is_empty() {
            crate::publish_notification_policy(app, shared, observed_at_unix_ms);
        }
        Ok(latest)
    }
}

#[derive(Clone, Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct HistoryRecoveryReport {
    pub scanned_rows: usize,
    pub recovered_events: usize,
    pub skipped_rows: usize,
    pub truncated: bool,
    pub last_source_sequence: Option<u64>,
}

fn restart_safe_update(update: BrokerUpdate) -> Option<BrokerUpdate> {
    match update {
        BrokerUpdate::UpsertSession(mut session)
            if session.identity.source == SourceKind::CodexCliObserver =>
        {
            if session.lifecycle != SessionLifecycle::Ended {
                session.lifecycle = SessionLifecycle::Unknown;
                session.activity = SessionActivity::Unknown;
                session.connection = ConnectionHealth::Unknown;
                session.waiting = WaitingState::Unknown;
            }
            Some(BrokerUpdate::UpsertSession(session))
        }
        BrokerUpdate::PatchSessionState(patch)
            if patch.identity.source == SourceKind::CodexCliObserver =>
        {
            Some(BrokerUpdate::PatchSessionState(patch))
        }
        BrokerUpdate::UpsertTurn(mut turn)
            if turn.identity.source == SourceKind::CodexCliObserver
                && turn.session_id.source == SourceKind::CodexCliObserver =>
        {
            if turn.lifecycle == TurnLifecycle::Active {
                turn.lifecycle = TurnLifecycle::Unknown;
                turn.outcome = TurnOutcome::Unknown;
            }
            Some(BrokerUpdate::UpsertTurn(turn))
        }
        BrokerUpdate::UpsertToolItem(mut tool)
            if tool.identity.source == SourceKind::CodexCliObserver
                && tool.session_id.source == SourceKind::CodexCliObserver =>
        {
            if tool.state == ToolItemState::Active {
                tool.state = ToolItemState::Unknown;
            }
            Some(BrokerUpdate::UpsertToolItem(tool))
        }
        _ => None,
    }
}

/// Accept only the first bounded set of exact Codex identities during replay.
/// Keeping the original typed IDs avoids inventing a second identity scheme.
fn within_recovery_limits(
    update: &BrokerUpdate,
    sessions: &mut BTreeSet<SourceScopedId>,
    turns: &mut BTreeSet<SourceScopedId>,
) -> bool {
    let (session_id, turn_id) = match update {
        BrokerUpdate::UpsertSession(session) => (Some(&session.identity), None),
        BrokerUpdate::PatchSessionState(patch) => (Some(&patch.identity), None),
        BrokerUpdate::UpsertTurn(turn) => (Some(&turn.session_id), Some(&turn.identity)),
        BrokerUpdate::UpsertToolItem(tool) => (Some(&tool.session_id), None),
        _ => return false,
    };
    let Some(session_id) = session_id else {
        return false;
    };
    if session_id.source != SourceKind::CodexCliObserver {
        return false;
    }
    if !sessions.contains(session_id) && sessions.len() >= MAX_MONITORED_SESSIONS {
        return false;
    }
    if let Some(turn_id) = turn_id {
        if turn_id.source != SourceKind::CodexCliObserver
            || (!turns.contains(turn_id) && turns.len() >= MAX_MONITORED_TURNS)
        {
            return false;
        }
    }
    sessions.insert(session_id.clone());
    if let Some(turn_id) = turn_id {
        turns.insert(turn_id.clone());
    }
    true
}

/// Normalize only the verified session/turn identifiers. Missing identifiers
/// yield no guessed state, and Stop always names its exact captured turn.
pub fn normalize_observation(
    observation: &CodexHookLifecycleObservation,
    observed_at_unix_ms: u64,
    terminal_sessions: &BTreeSet<String>,
    existing_session: Option<&Session>,
    existing_turn: Option<&Turn>,
    has_other_active_turn: bool,
) -> Vec<BrokerUpdate> {
    let Some(session_value) = observation.session_id.as_deref() else {
        return Vec::new();
    };
    let Ok(session_id) = OpaqueId::new(session_value.to_owned()) else {
        return Vec::new();
    };
    let identity = SourceScopedId::new(SourceKind::CodexCliObserver, session_id.clone());
    let terminal = terminal_sessions.contains(session_value)
        || existing_session.is_some_and(|session| session.lifecycle == SessionLifecycle::Ended);

    match observation.event {
        CodexHookEvent::SessionStart => {
            if terminal {
                return Vec::new();
            }
            let session = session_with_state(
                existing_session,
                identity,
                SessionLifecycle::Active,
                existing_session
                    .map(|session| session.activity)
                    .filter(|activity| *activity == SessionActivity::Working)
                    .unwrap_or(SessionActivity::Idle),
                Some(observed_at_unix_ms),
                observed_at_unix_ms,
            );
            vec![BrokerUpdate::UpsertSession(session)]
        }
        CodexHookEvent::UserPromptSubmit => {
            if terminal
                || existing_turn.is_some_and(|turn| {
                    turn.session_id == identity
                        && matches!(
                            turn.lifecycle,
                            TurnLifecycle::Completed
                                | TurnLifecycle::Interrupted
                                | TurnLifecycle::Failed
                        )
                })
            {
                return Vec::new();
            }
            let mut updates = vec![BrokerUpdate::UpsertSession(session_with_state(
                existing_session,
                identity.clone(),
                SessionLifecycle::Active,
                SessionActivity::Working,
                None,
                observed_at_unix_ms,
            ))];
            if let Some(turn_value) = observation.turn_id.as_deref() {
                if let Ok(turn_id) = OpaqueId::new(turn_value.to_owned()) {
                    let mut turn = existing_turn.cloned().unwrap_or_else(|| Turn {
                        schema_version: SchemaVersion::current(),
                        identity: SourceScopedId::new(SourceKind::CodexCliObserver, turn_id),
                        session_id: identity.clone(),
                        thread_id: None,
                        agent_id: None,
                        lifecycle: TurnLifecycle::Active,
                        outcome: TurnOutcome::Unknown,
                        started_at_unix_ms: Some(observed_at_unix_ms),
                        ended_at_unix_ms: None,
                        summary: None,
                        plan: None,
                    });
                    if turn.session_id == identity
                        && !existing_turn.is_some_and(|existing| {
                            matches!(
                                existing.lifecycle,
                                TurnLifecycle::Completed
                                    | TurnLifecycle::Interrupted
                                    | TurnLifecycle::Failed
                            )
                        })
                    {
                        turn.lifecycle = TurnLifecycle::Active;
                        turn.outcome = TurnOutcome::Unknown;
                        turn.started_at_unix_ms.get_or_insert(observed_at_unix_ms);
                        updates.push(BrokerUpdate::UpsertTurn(turn));
                    }
                }
            }
            updates
        }
        CodexHookEvent::Stop => {
            let mut updates = Vec::new();
            let stop_changes_turn = observation.turn_id.is_some()
                && existing_turn.is_none_or(|turn| {
                    turn.session_id == identity
                        && !matches!(
                            turn.lifecycle,
                            TurnLifecycle::Completed
                                | TurnLifecycle::Interrupted
                                | TurnLifecycle::Failed
                        )
                });
            if !terminal && stop_changes_turn {
                updates.push(BrokerUpdate::UpsertSession(session_with_state(
                    existing_session,
                    identity.clone(),
                    SessionLifecycle::Active,
                    if has_other_active_turn {
                        SessionActivity::Working
                    } else {
                        SessionActivity::Idle
                    },
                    None,
                    observed_at_unix_ms,
                )));
            }
            if let Some(turn_value) = observation.turn_id.as_deref() {
                if let Ok(turn_id) = OpaqueId::new(turn_value.to_owned()) {
                    let mut turn = existing_turn.cloned().unwrap_or_else(|| Turn {
                        schema_version: SchemaVersion::current(),
                        identity: SourceScopedId::new(SourceKind::CodexCliObserver, turn_id),
                        session_id: identity.clone(),
                        thread_id: None,
                        agent_id: None,
                        lifecycle: TurnLifecycle::Completed,
                        outcome: TurnOutcome::Unknown,
                        started_at_unix_ms: None,
                        ended_at_unix_ms: Some(observed_at_unix_ms),
                        summary: None,
                        plan: None,
                    });
                    if turn.session_id == identity
                        && !existing_turn.is_some_and(|existing| {
                            matches!(
                                existing.lifecycle,
                                TurnLifecycle::Completed
                                    | TurnLifecycle::Interrupted
                                    | TurnLifecycle::Failed
                            )
                        })
                    {
                        turn.lifecycle = TurnLifecycle::Completed;
                        turn.outcome = TurnOutcome::Unknown;
                        turn.ended_at_unix_ms = Some(observed_at_unix_ms);
                        updates.push(BrokerUpdate::UpsertTurn(turn));
                    }
                }
            }
            updates
        }
        CodexHookEvent::SessionEnd => vec![BrokerUpdate::UpsertSession(session_with_state(
            existing_session,
            identity,
            SessionLifecycle::Ended,
            SessionActivity::Unknown,
            None,
            observed_at_unix_ms,
        ))],
    }
}

fn session_with_state(
    existing: Option<&Session>,
    identity: SourceScopedId,
    lifecycle: SessionLifecycle,
    activity: SessionActivity,
    started_at_unix_ms: Option<u64>,
    observed_at_unix_ms: u64,
) -> Session {
    let mut session = existing.cloned().unwrap_or_else(|| Session {
        schema_version: SchemaVersion::current(),
        identity,
        thread_id: None,
        agent_id: None,
        parent_agent_id: None,
        repository_id: None,
        worktree_id: None,
        display_name: None,
        model: None,
        waiting: WaitingState::Unknown,
        connection: ConnectionHealth::Healthy,
        lifecycle: SessionLifecycle::Unknown,
        activity: SessionActivity::Unknown,
        started_at_unix_ms: None,
        last_seen_at_unix_ms: None,
    });
    session.lifecycle = lifecycle;
    session.activity = activity;
    session.connection = if lifecycle == SessionLifecycle::Ended {
        ConnectionHealth::Offline
    } else {
        ConnectionHealth::Healthy
    };
    if session.started_at_unix_ms.is_none() {
        session.started_at_unix_ms = started_at_unix_ms;
    }
    session.last_seen_at_unix_ms = Some(
        session
            .last_seen_at_unix_ms
            .unwrap_or(0)
            .max(observed_at_unix_ms),
    );
    session
}

fn redact_error(error: &str) -> String {
    let mut safe = error
        .chars()
        .filter(|character| !character.is_control())
        .take(240)
        .collect::<String>();
    for marker in ["C:\\", "\\\\", "/Users/", "/home/"] {
        if safe.contains(marker) {
            safe = "Observer reported a local error; details were omitted.".into();
            break;
        }
    }
    safe
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::broker::types::{
        EventCorrelation, EventEnvelope, EventSequence, PendingRequest, RequestKind,
        RequestLifecycle,
    };
    use crate::storage::HistoryStore;
    use codex_hook_contract::LIFECYCLE_WIRE_VERSION;
    use std::sync::atomic::{AtomicU64, Ordering};

    fn observation(
        event: CodexHookEvent,
        session: Option<&str>,
        turn: Option<&str>,
    ) -> CodexHookLifecycleObservation {
        CodexHookLifecycleObservation {
            wire_version: LIFECYCLE_WIRE_VERSION,
            event,
            session_id: session.map(str::to_owned),
            turn_id: turn.map(str::to_owned),
        }
    }

    fn recovery_path() -> std::path::PathBuf {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        std::env::current_dir()
            .unwrap()
            .join("target")
            .join("monitor-recovery-tests")
            .join(format!(
                "{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ))
            .join("history.sqlite3")
    }

    fn history_event(update: BrokerUpdate, sequence: u64) -> EventEnvelope<BrokerUpdate> {
        EventEnvelope {
            schema_version: SchemaVersion::current(),
            sequence: EventSequence::new(sequence).unwrap(),
            source: SourceKind::CodexCliObserver,
            source_event_id: None,
            observed_at_unix_ms: 1_000 + sequence,
            correlation: EventCorrelation::default(),
            deduplication_key: None,
            payload: update,
        }
    }

    #[test]
    fn lifecycle_maps_to_source_scoped_records_without_guessed_fields() {
        let terminal = BTreeSet::new();
        let start = normalize_observation(
            &observation(CodexHookEvent::SessionStart, Some("session:a"), None),
            10,
            &terminal,
            None,
            None,
            false,
        );
        assert!(matches!(
            &start[0],
            BrokerUpdate::UpsertSession(session)
                if session.identity.id.as_str() == "session:a"
                    && session.lifecycle == SessionLifecycle::Active
                    && session.thread_id.is_none()
                    && session.display_name.is_none()
        ));

        let prompt = normalize_observation(
            &observation(
                CodexHookEvent::UserPromptSubmit,
                Some("session:a"),
                Some("turn:old"),
            ),
            20,
            &terminal,
            None,
            None,
            false,
        );
        assert!(matches!(&prompt[0], BrokerUpdate::UpsertSession(session)
            if session.activity == SessionActivity::Working));
        assert!(matches!(&prompt[1], BrokerUpdate::UpsertTurn(turn)
            if turn.identity.id.as_str() == "turn:old"
                && turn.lifecycle == TurnLifecycle::Active
                && turn.outcome == TurnOutcome::Unknown));

        // A late Stop targets the captured old turn itself. It does not need a
        // current-turn slot and never creates a new turn identity.
        let active_turn = Turn {
            schema_version: SchemaVersion::current(),
            identity: SourceScopedId::new(
                SourceKind::CodexCliObserver,
                OpaqueId::new("turn:newer").unwrap(),
            ),
            session_id: SourceScopedId::new(
                SourceKind::CodexCliObserver,
                OpaqueId::new("session:a").unwrap(),
            ),
            thread_id: None,
            agent_id: None,
            lifecycle: TurnLifecycle::Active,
            outcome: TurnOutcome::Unknown,
            started_at_unix_ms: Some(25),
            ended_at_unix_ms: None,
            summary: None,
            plan: None,
        };
        let old_turn = match &prompt[1] {
            BrokerUpdate::UpsertTurn(turn) => turn,
            _ => unreachable!(),
        };
        let working_session = match &prompt[0] {
            BrokerUpdate::UpsertSession(session) => session,
            _ => unreachable!(),
        };
        let stop = normalize_observation(
            &observation(CodexHookEvent::Stop, Some("session:a"), Some("turn:old")),
            30,
            &terminal,
            Some(working_session),
            Some(old_turn),
            true,
        );
        assert!(matches!(&stop[0], BrokerUpdate::UpsertSession(session)
            if session.activity == SessionActivity::Working));
        assert!(matches!(&stop[1], BrokerUpdate::UpsertTurn(turn)
            if turn.identity.id.as_str() == "turn:old"
                && turn.lifecycle == TurnLifecycle::Completed
                && turn.outcome == TurnOutcome::Unknown));
        let completed_turn = Turn {
            lifecycle: TurnLifecycle::Completed,
            ..active_turn.clone()
        };
        assert!(normalize_observation(
            &observation(
                CodexHookEvent::UserPromptSubmit,
                Some("session:a"),
                Some("turn:newer")
            ),
            35,
            &terminal,
            None,
            Some(&completed_turn),
            false,
        )
        .is_empty());
        let working_session = Session {
            identity: SourceScopedId::new(
                SourceKind::CodexCliObserver,
                OpaqueId::new("session:a").unwrap(),
            ),
            activity: SessionActivity::Working,
            ..match &start[0] {
                BrokerUpdate::UpsertSession(session) => session.clone(),
                _ => unreachable!(),
            }
        };
        let duplicate_start = normalize_observation(
            &observation(CodexHookEvent::SessionStart, Some("session:a"), None),
            36,
            &terminal,
            Some(&working_session),
            None,
            false,
        );
        assert!(
            matches!(&duplicate_start[0], BrokerUpdate::UpsertSession(session)
            if session.activity == SessionActivity::Working)
        );
        assert!(normalize_observation(
            &observation(CodexHookEvent::Stop, None, Some("turn:old")),
            31,
            &terminal,
            None,
            None,
            false,
        )
        .is_empty());
    }

    #[test]
    fn terminal_sessions_do_not_resurrect_but_exact_old_turn_stop_is_recorded() {
        let terminal = BTreeSet::from(["session:a".to_owned()]);
        assert!(normalize_observation(
            &observation(
                CodexHookEvent::UserPromptSubmit,
                Some("session:a"),
                Some("turn:new")
            ),
            40,
            &terminal,
            None,
            None,
            false,
        )
        .is_empty());
        let old_turn = Turn {
            schema_version: SchemaVersion::current(),
            identity: SourceScopedId::new(
                SourceKind::CodexCliObserver,
                OpaqueId::new("turn:old").unwrap(),
            ),
            session_id: SourceScopedId::new(
                SourceKind::CodexCliObserver,
                OpaqueId::new("session:a").unwrap(),
            ),
            thread_id: None,
            agent_id: None,
            lifecycle: TurnLifecycle::Active,
            outcome: TurnOutcome::Unknown,
            started_at_unix_ms: Some(30),
            ended_at_unix_ms: None,
            summary: None,
            plan: None,
        };
        let stop = normalize_observation(
            &observation(CodexHookEvent::Stop, Some("session:a"), Some("turn:old")),
            50,
            &terminal,
            None,
            Some(&old_turn),
            false,
        );
        assert_eq!(stop.len(), 1);
        assert!(matches!(&stop[0], BrokerUpdate::UpsertTurn(turn)
            if turn.lifecycle == TurnLifecycle::Completed));
    }

    #[test]
    fn late_stop_keeps_session_working_while_another_turn_is_active() {
        let terminal = BTreeSet::new();
        let session = Session {
            identity: SourceScopedId::new(
                SourceKind::CodexCliObserver,
                OpaqueId::new("session:concurrent").unwrap(),
            ),
            lifecycle: SessionLifecycle::Active,
            activity: SessionActivity::Working,
            ..match normalize_observation(
                &observation(
                    CodexHookEvent::SessionStart,
                    Some("session:concurrent"),
                    None,
                ),
                100,
                &terminal,
                None,
                None,
                false,
            )
            .remove(0)
            {
                BrokerUpdate::UpsertSession(session) => session,
                _ => unreachable!(),
            }
        };
        let old_turn = Turn {
            schema_version: SchemaVersion::current(),
            identity: SourceScopedId::new(
                SourceKind::CodexCliObserver,
                OpaqueId::new("turn:old").unwrap(),
            ),
            session_id: session.identity.clone(),
            thread_id: None,
            agent_id: None,
            lifecycle: TurnLifecycle::Active,
            outcome: TurnOutcome::Unknown,
            started_at_unix_ms: Some(101),
            ended_at_unix_ms: None,
            summary: None,
            plan: None,
        };
        let stop = normalize_observation(
            &observation(
                CodexHookEvent::Stop,
                Some("session:concurrent"),
                Some("turn:old"),
            ),
            200,
            &terminal,
            Some(&session),
            Some(&old_turn),
            true,
        );
        assert_eq!(stop.len(), 2);
        assert!(matches!(&stop[0], BrokerUpdate::UpsertSession(session)
            if session.activity == SessionActivity::Working));
        assert!(matches!(&stop[1], BrokerUpdate::UpsertTurn(turn)
            if turn.lifecycle == TurnLifecycle::Completed));
    }

    #[test]
    fn default_status_is_truthfully_limited_and_never_production_enabled() {
        let monitor = CodexMonitor::default();
        let status = monitor.status();
        assert!(status.pipeline_available);
        assert!(!status.production_enabled);
        assert_eq!(status.runtime_version, None);
        assert_eq!(status.capability, "experimental_disabled");
        monitor.set_observer_state(ObserverState::Listening, None);
        assert!(!monitor.status().production_enabled);
    }

    #[test]
    fn hook_status_exposes_registration_and_relay_without_opening_live_gate() {
        let monitor = CodexMonitor::default();
        let events = CodexHookEvent::VERIFIED
            .iter()
            .map(|event| event.as_str().to_string())
            .collect::<Vec<_>>();
        monitor.set_hook_readiness(Some("codex-cli 0.157.1".into()), true, events.clone(), true);
        let status = monitor.status();
        assert_eq!(status.schema_version, 2);
        assert_eq!(status.runtime_version.as_deref(), Some("codex-cli 0.157.1"));
        assert_eq!(status.configured_hook_events, events);
        assert!(status.hook_registration_verified);
        assert!(status.relay_ready);
        assert!(!status.cross_session_identity_verified);
        assert!(!status.production_enabled);

        monitor.set_observer_state(ObserverState::Listening, None);
        assert!(!monitor.status().production_enabled);
        monitor.state.lock().unwrap().evidence.history_ready = true;
        assert!(!monitor.status().production_enabled);

        let mut incomplete_events = events;
        incomplete_events.pop();
        monitor.set_hook_readiness(
            Some("codex-cli 0.157.1".into()),
            true,
            incomplete_events,
            true,
        );
        assert!(!monitor.status().hook_registration_verified);
        assert!(!monitor.status().production_enabled);

        monitor.set_hook_readiness(
            Some("codex-cli 0.157.1".into()),
            false,
            CodexHookEvent::VERIFIED
                .iter()
                .map(|event| event.as_str().to_string())
                .collect(),
            true,
        );
        assert!(!monitor.status().production_enabled);

        monitor.set_hook_readiness(
            Some("codex-cli 0.157.1".into()),
            true,
            CodexHookEvent::VERIFIED
                .iter()
                .map(|event| event.as_str().to_string())
                .collect(),
            false,
        );
        assert!(!monitor.status().hook_registration_verified);
        assert!(!monitor.status().relay_ready);
        assert!(!monitor.status().production_enabled);
        monitor.set_observer_state(ObserverState::Degraded, None);
        assert!(!monitor.status().production_enabled);
    }

    #[test]
    fn restart_recovery_restores_stale_state_and_never_replays_pending_requests() {
        let path = recovery_path();
        let mut store = HistoryStore::open(&path).unwrap();
        let active_session = Session {
            identity: SourceScopedId::new(
                SourceKind::CodexCliObserver,
                OpaqueId::new("session:restore").unwrap(),
            ),
            ..match normalize_observation(
                &observation(CodexHookEvent::SessionStart, Some("session:restore"), None),
                1_000,
                &BTreeSet::new(),
                None,
                None,
                false,
            )
            .remove(0)
            {
                BrokerUpdate::UpsertSession(session) => session,
                _ => unreachable!(),
            }
        };
        let turn = Turn {
            schema_version: SchemaVersion::current(),
            identity: SourceScopedId::new(
                SourceKind::CodexCliObserver,
                OpaqueId::new("turn:restore").unwrap(),
            ),
            session_id: active_session.identity.clone(),
            thread_id: None,
            agent_id: None,
            lifecycle: TurnLifecycle::Active,
            outcome: TurnOutcome::Unknown,
            started_at_unix_ms: Some(1_005),
            ended_at_unix_ms: None,
            summary: None,
            plan: None,
        };
        let ended_session = Session {
            identity: SourceScopedId::new(
                SourceKind::CodexCliObserver,
                OpaqueId::new("session:ended").unwrap(),
            ),
            lifecycle: SessionLifecycle::Ended,
            activity: SessionActivity::Unknown,
            connection: ConnectionHealth::Offline,
            ..active_session.clone()
        };
        let request = PendingRequest {
            schema_version: SchemaVersion::current(),
            identity: SourceScopedId::new(
                SourceKind::CodexCliObserver,
                OpaqueId::new("request:never-replay").unwrap(),
            ),
            session_id: Some(active_session.identity.clone()),
            thread_id: None,
            turn_id: None,
            tool_item_id: None,
            agent_id: None,
            kind: RequestKind::Unknown,
            requested_capability: None,
            lifecycle: RequestLifecycle::Pending,
            created_at_unix_ms: Some(1_006),
            deadline_unix_ms: None,
        };
        store
            .record_broker_events(
                &[
                    history_event(BrokerUpdate::UpsertSession(active_session.clone()), 100),
                    history_event(BrokerUpdate::UpsertTurn(turn), 101),
                    history_event(BrokerUpdate::UpsertPendingRequest(request), 102),
                    history_event(BrokerUpdate::UpsertSession(ended_session), 103),
                ],
                false,
            )
            .unwrap();
        drop(store);

        let mut reopened = HistoryStore::open(&path).unwrap();
        let monitor = CodexMonitor::default();
        let broker = BrokerService::default();
        let report = monitor.recover_history(&reopened, &broker).unwrap();
        assert_eq!(report.recovered_events, 3);
        assert_eq!(report.skipped_rows, 0);
        let sync = broker.synchronize_after(u64::MAX).unwrap();
        let crate::broker::stream::BrokerSyncResponse::Snapshot { snapshot } = sync else {
            unreachable!()
        };
        let restored = snapshot
            .sessions
            .iter()
            .find(|session| session.identity.id.as_str() == "session:restore")
            .unwrap();
        assert_eq!(restored.lifecycle, SessionLifecycle::Unknown);
        assert_eq!(restored.activity, SessionActivity::Unknown);
        assert_eq!(restored.connection, ConnectionHealth::Unknown);
        let ended = snapshot
            .sessions
            .iter()
            .find(|session| session.identity.id.as_str() == "session:ended")
            .unwrap();
        assert_eq!(ended.lifecycle, SessionLifecycle::Ended);
        let restored_turn = snapshot
            .turns
            .iter()
            .find(|turn| turn.identity.id.as_str() == "turn:restore")
            .unwrap();
        assert_eq!(restored_turn.lifecycle, TurnLifecycle::Unknown);
        assert!(snapshot.pending_requests.is_empty());
        assert!(monitor.status().broker_state_recovered);
        assert_eq!(monitor.status().recovered_event_count, 3);

        let mut live_session = restored.clone();
        live_session.lifecycle = SessionLifecycle::Ended;
        live_session.activity = SessionActivity::Unknown;
        live_session.waiting = WaitingState::Unknown;
        live_session.connection = ConnectionHealth::Unknown;
        live_session.last_seen_at_unix_ms = Some(2_000);
        let live = broker
            .apply_batch(vec![BrokerUpdate::UpsertSession(live_session)], 2_000)
            .unwrap();
        let live_sequence = live.events[0].sequence.get();
        assert!(live_sequence > 103);
        reopened.record_broker_events(&live.events, false).unwrap();
        drop(snapshot);
        drop(reopened);

        let second_history = HistoryStore::open(&path).unwrap();
        let second_monitor = CodexMonitor::default();
        let second_broker = BrokerService::default();
        let second_report = second_monitor
            .recover_history(&second_history, &second_broker)
            .unwrap();
        assert_eq!(second_report.last_source_sequence, Some(live_sequence));
        let second_sync = second_broker.synchronize_after(u64::MAX).unwrap();
        let crate::broker::stream::BrokerSyncResponse::Snapshot {
            snapshot: second_snapshot,
        } = second_sync
        else {
            unreachable!()
        };
        let second_restored = second_snapshot
            .sessions
            .iter()
            .find(|session| session.identity.id.as_str() == "session:restore")
            .unwrap();
        assert_eq!(second_restored.lifecycle, SessionLifecycle::Ended);
        assert!(second_snapshot.sequence >= live_sequence);

        drop(second_history);
        std::fs::remove_dir_all(path.parent().unwrap().parent().unwrap()).unwrap();
    }
}
