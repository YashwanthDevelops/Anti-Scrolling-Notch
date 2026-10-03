//! Sequenced snapshots and bounded in-memory replay for the backend broker.
//!
//! This module is the synchronization contract for a future Tauri view store.
//! It does not connect to the UI or interpret upstream Codex events. Producers
//! supply normalized updates and their observation timestamp.

use std::collections::{HashSet, VecDeque};
use std::num::NonZeroUsize;

use serde::Serialize;

use super::reducer::{ApplyOutcome, BrokerState, BrokerUpdate};
use super::types::{
    EventCorrelation, EventEnvelope, EventSequence, Integration, OpaqueId, PendingRequest,
    Repository, SchemaVersion, Session, SourceKind, ToolItem, Turn,
};

pub const DEFAULT_REPLAY_CAPACITY: usize = 256;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ApplyReceipt {
    pub outcome: ApplyOutcome,
    /// Zero means that the stream has not accepted a state-changing update.
    pub sequence: u64,
    /// True only when an existing source event ID or deduplication key matched
    /// an entry still retained in the bounded deduplication window.
    pub deduplicated: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StreamError {
    SequenceExhausted,
}

impl std::fmt::Display for StreamError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SequenceExhausted => formatter.write_str("broker event sequence is exhausted"),
        }
    }
}

impl std::error::Error for StreamError {}

/// Complete backend state at one sequence cursor. The cursor is zero only for
/// the initial empty state; emitted event sequences are always non-zero.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BrokerSnapshot {
    pub schema_version: SchemaVersion,
    pub sequence: u64,
    pub sessions: Vec<Session>,
    pub turns: Vec<Turn>,
    pub tool_items: Vec<ToolItem>,
    pub pending_requests: Vec<PendingRequest>,
    pub repositories: Vec<Repository>,
    pub integrations: Vec<Integration>,
}

/// A consumer sends its last applied sequence. A fresh UI (cursor zero), a
/// future cursor, or a cursor older than retained replay receives a snapshot.
/// Otherwise the response is a contiguous replay through the current head.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
pub enum BrokerSyncResponse {
    Snapshot {
        snapshot: BrokerSnapshot,
    },
    Replay {
        after_sequence: u64,
        through_sequence: u64,
        events: Vec<EventEnvelope<BrokerUpdate>>,
    },
}

/// Serializes state-changing normalized updates, retains a bounded recent
/// replay, and can repair a consumer cursor with a full state snapshot.
#[derive(Clone, Debug)]
pub struct BrokerStream {
    state: BrokerState,
    sequence: u64,
    replay: VecDeque<EventEnvelope<BrokerUpdate>>,
    replay_capacity: NonZeroUsize,
    seen_source_events: DeduplicationWindow,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
enum IdentifierKind {
    SourceEvent,
    Deduplication,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct ScopedEventIdentifier {
    source: SourceKind,
    kind: IdentifierKind,
    value: OpaqueId,
}

/// Remembers at most one replay window of source events. A record can retain
/// both identifiers for one event, so the identifier set is bounded at twice
/// the record capacity. Missing source IDs never fall back to payload hashing.
#[derive(Clone, Debug)]
struct DeduplicationWindow {
    capacity: usize,
    identifiers: HashSet<ScopedEventIdentifier>,
    events: VecDeque<Vec<ScopedEventIdentifier>>,
}

impl DeduplicationWindow {
    fn new(capacity: NonZeroUsize) -> Self {
        Self {
            capacity: capacity.get(),
            identifiers: HashSet::with_capacity(capacity.get().saturating_mul(2)),
            events: VecDeque::with_capacity(capacity.get()),
        }
    }

    fn duplicate_or_remember(
        &mut self,
        source: SourceKind,
        source_event_id: Option<&OpaqueId>,
        deduplication_key: Option<&OpaqueId>,
    ) -> bool {
        let mut identifiers = Vec::with_capacity(2);
        if let Some(value) = source_event_id {
            identifiers.push(ScopedEventIdentifier {
                source,
                kind: IdentifierKind::SourceEvent,
                value: value.clone(),
            });
        }
        if let Some(value) = deduplication_key {
            identifiers.push(ScopedEventIdentifier {
                source,
                kind: IdentifierKind::Deduplication,
                value: value.clone(),
            });
        }
        if identifiers.is_empty() {
            return false;
        }
        if identifiers
            .iter()
            .any(|identifier| self.identifiers.contains(identifier))
        {
            return true;
        }

        for identifier in &identifiers {
            self.identifiers.insert(identifier.clone());
        }
        self.events.push_back(identifiers);
        while self.events.len() > self.capacity {
            if let Some(expired) = self.events.pop_front() {
                for identifier in expired {
                    self.identifiers.remove(&identifier);
                }
            }
        }
        false
    }
}

impl Default for BrokerStream {
    fn default() -> Self {
        Self::with_replay_capacity(
            NonZeroUsize::new(DEFAULT_REPLAY_CAPACITY)
                .expect("default replay capacity is non-zero"),
        )
    }
}

impl BrokerStream {
    /// Use a smaller replay window for constrained callers/tests; callers
    /// cannot increase the fixed 256-event memory bound.
    pub fn with_replay_capacity(replay_capacity: NonZeroUsize) -> Self {
        let replay_capacity = NonZeroUsize::new(replay_capacity.get().min(DEFAULT_REPLAY_CAPACITY))
            .expect("default replay capacity is non-zero");
        Self {
            state: BrokerState::default(),
            sequence: 0,
            replay: VecDeque::with_capacity(replay_capacity.get()),
            replay_capacity,
            seen_source_events: DeduplicationWindow::new(replay_capacity),
        }
    }

    pub fn sequence(&self) -> u64 {
        self.sequence
    }

    pub fn state(&self) -> &BrokerState {
        &self.state
    }

    pub fn apply(
        &mut self,
        update: BrokerUpdate,
        observed_at_unix_ms: u64,
    ) -> Result<ApplyReceipt, StreamError> {
        self.apply_with_source_ids(update, observed_at_unix_ms, None, None)
    }

    /// Apply a normalized update with identifiers already supplied by its
    /// adapter. The broker scopes each identifier by source and identifier
    /// kind. It does not infer IDs from payload content or anonymous hook names.
    pub fn apply_with_source_ids(
        &mut self,
        update: BrokerUpdate,
        observed_at_unix_ms: u64,
        source_event_id: Option<OpaqueId>,
        deduplication_key: Option<OpaqueId>,
    ) -> Result<ApplyReceipt, StreamError> {
        // Fail before mutation rather than wrapping or changing state without
        // a sequence if the monotonic cursor has reached its maximum value.
        if self.sequence == u64::MAX {
            return Err(StreamError::SequenceExhausted);
        }

        let source = update.source();
        if self.seen_source_events.duplicate_or_remember(
            source,
            source_event_id.as_ref(),
            deduplication_key.as_ref(),
        ) {
            return Ok(ApplyReceipt {
                outcome: ApplyOutcome::Unchanged,
                sequence: self.sequence,
                deduplicated: true,
            });
        }

        let outcome = self.state.apply(update.clone());
        if outcome == ApplyOutcome::Unchanged {
            return Ok(ApplyReceipt {
                outcome,
                sequence: self.sequence,
                deduplicated: false,
            });
        }

        let sequence = self.sequence + 1;
        let event = EventEnvelope {
            schema_version: SchemaVersion::current(),
            sequence: EventSequence::new(sequence)
                .expect("a sequenced broker event always has a non-zero sequence"),
            source,
            source_event_id,
            observed_at_unix_ms,
            correlation: EventCorrelation::default(),
            deduplication_key,
            payload: update,
        };
        self.sequence = sequence;
        self.replay.push_back(event);
        if self.replay.len() > self.replay_capacity.get() {
            self.replay.pop_front();
        }

        Ok(ApplyReceipt {
            outcome,
            sequence,
            deduplicated: false,
        })
    }

    pub fn snapshot(&self) -> BrokerSnapshot {
        BrokerSnapshot {
            schema_version: SchemaVersion::current(),
            sequence: self.sequence,
            sessions: self.state.sessions().cloned().collect(),
            turns: self.state.turns().cloned().collect(),
            tool_items: self.state.tool_items().cloned().collect(),
            pending_requests: self.state.pending_requests().cloned().collect(),
            repositories: self.state.repositories().cloned().collect(),
            integrations: self.state.integrations().cloned().collect(),
        }
    }

    pub fn synchronize_after(&self, after_sequence: u64) -> BrokerSyncResponse {
        if after_sequence == 0 || after_sequence > self.sequence {
            return self.snapshot_response();
        }
        if after_sequence == self.sequence {
            return BrokerSyncResponse::Replay {
                after_sequence,
                through_sequence: self.sequence,
                events: Vec::new(),
            };
        }

        let expected_first = after_sequence + 1;
        let events = self
            .replay
            .iter()
            .filter(|event| event.sequence.get() > after_sequence)
            .cloned()
            .collect::<Vec<_>>();
        let mut previous_sequence = after_sequence;
        let contiguous = events.iter().all(|event| {
            let Some(expected) = previous_sequence.checked_add(1) else {
                return false;
            };
            if event.sequence.get() != expected {
                return false;
            }
            previous_sequence = expected;
            true
        });
        let complete = contiguous
            && events.first().map(|event| event.sequence.get()) == Some(expected_first)
            && events.last().map(|event| event.sequence.get()) == Some(self.sequence)
            && events.len() as u64 == self.sequence - after_sequence;

        if !complete {
            return self.snapshot_response();
        }

        BrokerSyncResponse::Replay {
            after_sequence,
            through_sequence: self.sequence,
            events,
        }
    }

    fn snapshot_response(&self) -> BrokerSyncResponse {
        BrokerSyncResponse::Snapshot {
            snapshot: self.snapshot(),
        }
    }
}

impl BrokerUpdate {
    fn source(&self) -> SourceKind {
        match self {
            Self::UpsertSession(value) => value.identity.source,
            Self::PatchSessionState(value) => value.identity.source,
            Self::UpsertTurn(value) => value.identity.source,
            Self::UpsertToolItem(value) => value.identity.source,
            Self::UpsertPendingRequest(value) => value.identity.source,
            Self::UpsertRepository(value) => value.identity.source,
            Self::UpsertIntegration(_) => SourceKind::Integration,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroUsize;

    use super::*;
    use crate::broker::reducer::{BrokerUpdate, SessionStatePatch};
    use crate::broker::types::{
        ConnectionHealth, DOMAIN_SCHEMA_VERSION, OpaqueId, RequestKind, RequestLifecycle,
        SessionActivity, SessionLifecycle, SourceScopedId, WaitingState,
    };

    fn id(value: &str) -> OpaqueId {
        OpaqueId::new(value).unwrap()
    }

    fn session(identity: &str) -> Session {
        session_from_source(SourceKind::CodexCliObserver, identity)
    }

    fn session_from_source(source: SourceKind, identity: &str) -> Session {
        Session {
            schema_version: SchemaVersion::current(),
            identity: super::super::types::SourceScopedId::new(source, id(identity)),
            thread_id: None,
            agent_id: None,
            parent_agent_id: None,
            repository_id: None,
            worktree_id: None,
            display_name: None,
            model: None,
            lifecycle: SessionLifecycle::Unknown,
            activity: SessionActivity::Unknown,
            waiting: WaitingState::Unknown,
            connection: ConnectionHealth::Unknown,
            started_at_unix_ms: None,
            last_seen_at_unix_ms: None,
        }
    }

    fn pending_request(identity: &str) -> PendingRequest {
        PendingRequest {
            schema_version: SchemaVersion::current(),
            identity: SourceScopedId::new(SourceKind::CodexCliObserver, id(identity)),
            session_id: Some(SourceScopedId::new(
                SourceKind::CodexCliObserver,
                id("session-1"),
            )),
            thread_id: Some(id("thread-1")),
            turn_id: Some(id("turn-1")),
            tool_item_id: Some(id("tool-1")),
            agent_id: None,
            kind: RequestKind::Approval,
            requested_capability: Some(id("filesystem.write")),
            lifecycle: RequestLifecycle::Pending,
            created_at_unix_ms: Some(100),
            deadline_unix_ms: Some(1_000),
        }
    }

    fn small_stream(capacity: usize) -> BrokerStream {
        BrokerStream::with_replay_capacity(NonZeroUsize::new(capacity).unwrap())
    }

    #[test]
    fn snapshot_and_replay_cursors_share_the_current_monotonic_sequence() {
        let mut stream = BrokerStream::default();
        let empty = stream.snapshot();
        assert_eq!(empty.schema_version.get(), DOMAIN_SCHEMA_VERSION);
        assert_eq!(empty.sequence, 0);
        assert!(empty.sessions.is_empty());
        assert!(matches!(
            stream.synchronize_after(0),
            BrokerSyncResponse::Snapshot { snapshot } if snapshot.sequence == 0
        ));

        let first = session("session-1");
        let receipt = stream
            .apply(BrokerUpdate::UpsertSession(first.clone()), 100)
            .unwrap();
        assert_eq!(receipt.outcome, ApplyOutcome::Changed);
        assert_eq!(receipt.sequence, 1);
        assert_eq!(stream.sequence(), 1);

        let duplicate = stream
            .apply(BrokerUpdate::UpsertSession(first), 101)
            .unwrap();
        assert_eq!(duplicate.outcome, ApplyOutcome::Unchanged);
        assert_eq!(duplicate.sequence, 1);
        assert_eq!(stream.sequence(), 1);
        assert!(matches!(
            stream.synchronize_after(1),
            BrokerSyncResponse::Replay { events, through_sequence: 1, .. } if events.is_empty()
        ));

        let snapshot = serde_json::to_value(stream.snapshot()).unwrap();
        assert_eq!(snapshot["schemaVersion"], DOMAIN_SCHEMA_VERSION);
        assert_eq!(snapshot["sequence"], 1);
        assert_eq!(snapshot["sessions"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn terminal_request_update_replays_while_snapshots_omit_the_request() {
        let mut stream = BrokerStream::default();
        let pending = pending_request("request-1");
        let identity = pending.identity.clone();
        let inserted = stream
            .apply(BrokerUpdate::UpsertPendingRequest(pending.clone()), 100)
            .unwrap();
        assert_eq!(inserted.sequence, 1);
        assert_eq!(stream.snapshot().pending_requests, vec![pending.clone()]);

        let mut resolved = pending;
        resolved.lifecycle = RequestLifecycle::Resolved;
        let removed = stream
            .apply(BrokerUpdate::UpsertPendingRequest(resolved), 110)
            .unwrap();
        assert_eq!(removed.outcome, ApplyOutcome::Changed);
        assert_eq!(removed.sequence, 2);
        assert!(stream.snapshot().pending_requests.is_empty());
        assert!(stream.state().pending_request(&identity).is_none());

        let BrokerSyncResponse::Replay {
            after_sequence,
            through_sequence,
            events,
        } = stream.synchronize_after(1)
        else {
            panic!("the terminal update should be retained for replay");
        };
        assert_eq!(after_sequence, 1);
        assert_eq!(through_sequence, 2);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].sequence.get(), 2);
        let event = serde_json::to_value(&events[0]).unwrap();
        assert_eq!(event["payload"]["kind"], "upsert_pending_request");
        assert_eq!(event["payload"]["value"]["lifecycle"], "resolved");
    }

    #[test]
    fn replay_contains_contiguous_serializable_updates_and_their_source() {
        let mut stream = BrokerStream::default();
        let identity = session("session-1").identity;
        stream
            .apply(BrokerUpdate::UpsertSession(session("session-1")), 1_000)
            .unwrap();
        stream
            .apply(
                BrokerUpdate::PatchSessionState(SessionStatePatch {
                    identity,
                    lifecycle: Some(SessionLifecycle::Active),
                    activity: None,
                    waiting: None,
                    connection: None,
                }),
                1_001,
            )
            .unwrap();

        let response = stream.synchronize_after(1);
        let response_json = serde_json::to_value(&response).unwrap();
        assert_eq!(response_json["kind"], "replay");
        assert_eq!(response_json["afterSequence"], 1);
        assert_eq!(response_json["throughSequence"], 2);
        let BrokerSyncResponse::Replay {
            after_sequence,
            through_sequence,
            events,
        } = response
        else {
            panic!("a retained cursor should receive replay");
        };
        assert_eq!(after_sequence, 1);
        assert_eq!(through_sequence, 2);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].sequence.get(), 2);
        assert_eq!(events[0].source, SourceKind::CodexCliObserver);
        assert_eq!(events[0].observed_at_unix_ms, 1_001);
        assert!(events[0].correlation.session_id.is_none());
        let serialized = serde_json::to_value(&events[0]).unwrap();
        assert_eq!(serialized["schemaVersion"], DOMAIN_SCHEMA_VERSION);
        assert_eq!(serialized["sequence"], 2);
        assert_eq!(serialized["payload"]["kind"], "patch_session_state");
    }

    #[test]
    fn fresh_stale_and_future_cursors_receive_a_snapshot() {
        let mut stream = small_stream(2);
        for identity in ["session-1", "session-2", "session-3"] {
            stream
                .apply(BrokerUpdate::UpsertSession(session(identity)), 10)
                .unwrap();
        }

        assert!(matches!(
            stream.synchronize_after(0),
            BrokerSyncResponse::Snapshot { snapshot } if snapshot.sequence == 3 && snapshot.sessions.len() == 3
        ));
        assert!(matches!(
            stream.synchronize_after(4),
            BrokerSyncResponse::Snapshot { snapshot } if snapshot.sequence == 3
        ));

        let BrokerSyncResponse::Replay {
            after_sequence,
            through_sequence,
            events,
        } = stream.synchronize_after(1)
        else {
            panic!("the retained cursor should replay");
        };
        assert_eq!(after_sequence, 1);
        assert_eq!(through_sequence, 3);
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].sequence.get(), 2);
        assert_eq!(events[1].sequence.get(), 3);
    }

    #[test]
    fn an_internal_replay_gap_falls_back_to_a_complete_snapshot() {
        let mut stream = BrokerStream::default();
        for identity in ["session-1", "session-2", "session-3", "session-4"] {
            stream
                .apply(BrokerUpdate::UpsertSession(session(identity)), 10)
                .unwrap();
        }
        stream.replay.retain(|event| event.sequence.get() != 3);

        assert!(matches!(
            stream.synchronize_after(1),
            BrokerSyncResponse::Snapshot { snapshot }
                if snapshot.sequence == 4 && snapshot.sessions.len() == 4
        ));
    }

    #[test]
    fn sequence_exhaustion_fails_before_mutating_state() {
        let mut stream = BrokerStream {
            sequence: u64::MAX,
            ..BrokerStream::default()
        };
        let error = stream
            .apply(BrokerUpdate::UpsertSession(session("session-1")), 1)
            .unwrap_err();
        assert_eq!(error, StreamError::SequenceExhausted);
        assert_eq!(stream.sequence(), u64::MAX);
        assert_eq!(stream.state().sessions().count(), 0);
    }

    #[test]
    fn snapshots_keep_all_current_entity_families() {
        let stream = BrokerStream::default();
        let snapshot = stream.snapshot();
        let json = serde_json::to_value(snapshot).unwrap();
        for field in [
            "sessions",
            "turns",
            "toolItems",
            "pendingRequests",
            "repositories",
            "integrations",
        ] {
            assert!(json[field].is_array(), "snapshot field {field} is missing");
        }
    }

    #[test]
    fn requested_replay_capacity_cannot_exceed_the_global_bound() {
        let stream = BrokerStream::with_replay_capacity(NonZeroUsize::new(usize::MAX).unwrap());
        assert!(stream.replay.capacity() <= DEFAULT_REPLAY_CAPACITY);
    }

    #[test]
    fn repeated_source_event_ids_are_ignored_before_the_reducer_runs() {
        let mut stream = BrokerStream::default();
        let accepted = stream
            .apply_with_source_ids(
                BrokerUpdate::UpsertSession(session("first")),
                10,
                Some(id("event-1")),
                None,
            )
            .unwrap();
        let duplicate = stream
            .apply_with_source_ids(
                BrokerUpdate::UpsertSession(session("second")),
                11,
                Some(id("event-1")),
                None,
            )
            .unwrap();

        assert!(!accepted.deduplicated);
        assert_eq!(duplicate.outcome, ApplyOutcome::Unchanged);
        assert!(duplicate.deduplicated);
        assert_eq!(duplicate.sequence, 1);
        assert!(stream.state().session(&session("first").identity).is_some());
        assert!(stream
            .state()
            .session(&session("second").identity)
            .is_none());
        assert_eq!(stream.replay.len(), 1);
    }

    #[test]
    fn identical_source_ids_from_different_sources_remain_distinct() {
        let mut stream = BrokerStream::default();
        let codex = stream
            .apply_with_source_ids(
                BrokerUpdate::UpsertSession(session_from_source(
                    SourceKind::CodexCliObserver,
                    "codex-session",
                )),
                10,
                Some(id("same-id")),
                None,
            )
            .unwrap();
        let github = stream
            .apply_with_source_ids(
                BrokerUpdate::UpsertSession(session_from_source(SourceKind::GitHub, "pr-1")),
                11,
                Some(id("same-id")),
                None,
            )
            .unwrap();

        assert!(!codex.deduplicated);
        assert!(!github.deduplicated);
        assert_eq!(stream.sequence(), 2);
        assert_eq!(stream.state().sessions().count(), 2);
    }

    #[test]
    fn deduplication_keys_are_source_scoped_and_kind_scoped() {
        let mut stream = BrokerStream::default();
        stream
            .apply_with_source_ids(
                BrokerUpdate::UpsertSession(session("first")),
                10,
                None,
                Some(id("same-text")),
            )
            .unwrap();
        let duplicate = stream
            .apply_with_source_ids(
                BrokerUpdate::UpsertSession(session("duplicate")),
                11,
                None,
                Some(id("same-text")),
            )
            .unwrap();
        let distinct_kind = stream
            .apply_with_source_ids(
                BrokerUpdate::UpsertSession(session("different-kind")),
                12,
                Some(id("same-text")),
                None,
            )
            .unwrap();
        let distinct_source = stream
            .apply_with_source_ids(
                BrokerUpdate::UpsertSession(session_from_source(SourceKind::GitHub, "github")),
                13,
                None,
                Some(id("same-text")),
            )
            .unwrap();

        assert!(duplicate.deduplicated);
        assert!(!distinct_kind.deduplicated);
        assert!(!distinct_source.deduplicated);
        assert_eq!(stream.sequence(), 3);
        assert_eq!(stream.state().sessions().count(), 3);
    }

    #[test]
    fn missing_source_ids_do_not_trigger_payload_based_deduplication() {
        let mut stream = BrokerStream::default();
        let first = stream
            .apply(BrokerUpdate::UpsertSession(session("first")), 10)
            .unwrap();
        let second = stream
            .apply(BrokerUpdate::UpsertSession(session("second")), 11)
            .unwrap();

        assert!(!first.deduplicated);
        assert!(!second.deduplicated);
        assert_eq!(stream.sequence(), 2);
        assert_eq!(stream.state().sessions().count(), 2);
    }

    #[test]
    fn source_identity_window_is_bounded_and_evicts_oldest_events() {
        let mut stream = small_stream(2);
        for index in 1..=3 {
            stream
                .apply_with_source_ids(
                    BrokerUpdate::UpsertSession(session(&format!("session-{index}"))),
                    index,
                    Some(id(&format!("event-{index}"))),
                    None,
                )
                .unwrap();
        }
        let after_eviction = stream
            .apply_with_source_ids(
                BrokerUpdate::UpsertSession(session("session-after-eviction")),
                4,
                Some(id("event-1")),
                None,
            )
            .unwrap();

        assert!(!after_eviction.deduplicated);
        assert_eq!(after_eviction.sequence, 4);
        assert_eq!(stream.seen_source_events.events.len(), 2);
        assert_eq!(stream.seen_source_events.identifiers.len(), 2);
        assert!(stream
            .state()
            .session(&session("session-after-eviction").identity)
            .is_some());
    }

    #[test]
    fn accepted_source_identifiers_are_preserved_in_replay_envelopes() {
        let mut stream = BrokerStream::default();
        stream
            .apply_with_source_ids(
                BrokerUpdate::UpsertSession(session("session-1")),
                10,
                Some(id("source-event-1")),
                Some(id("dedupe-1")),
            )
            .unwrap();

        assert_eq!(stream.replay[0].source_event_id, Some(id("source-event-1")));
        assert_eq!(stream.replay[0].deduplication_key, Some(id("dedupe-1")));
    }
}
