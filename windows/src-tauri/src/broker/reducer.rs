//! Deterministic, in-memory reduction of normalized backend records.
//!
//! This is an internal state contract, not a Codex hook event schema. The
//! captured hook observation is anonymous and is deliberately not translated
//! into a session, turn, tool or request here. Sequence ordering, replay,
//! persistence, deduplication keys, timers and request resolution belong to
//! later Stage 5 tasks.

use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;

use super::types::{
    ConnectionHealth, Integration, OpaqueId, PendingRequest, Repository, SchemaVersion, Session,
    SessionActivity, SessionLifecycle, SourceKind, SourceScopedId, ToolItem, Turn, WaitingState,
};

type RelationIndex = BTreeMap<SourceScopedId, BTreeSet<SourceScopedId>>;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ApplyOutcome {
    Changed,
    Unchanged,
}

/// Independent session-state axes. `None` means leave that axis untouched;
/// `Some(Unknown)` explicitly resets it to unknown.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionStatePatch {
    pub identity: SourceScopedId,
    pub lifecycle: Option<SessionLifecycle>,
    pub activity: Option<SessionActivity>,
    pub waiting: Option<WaitingState>,
    pub connection: Option<ConnectionHealth>,
}

impl SessionStatePatch {
    fn is_empty(&self) -> bool {
        self.lifecycle.is_none()
            && self.activity.is_none()
            && self.waiting.is_none()
            && self.connection.is_none()
    }

    fn apply_to(&self, session: &mut Session) {
        if let Some(value) = self.lifecycle {
            session.lifecycle = value;
        }
        if let Some(value) = self.activity {
            session.activity = value;
        }
        if let Some(value) = self.waiting {
            session.waiting = value;
        }
        if let Some(value) = self.connection {
            session.connection = value;
        }
    }
}

/// Internal normalized state mutations. These variants do not describe
/// upstream Codex events and are not exposed over Tauri IPC.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum BrokerUpdate {
    UpsertSession(Session),
    PatchSessionState(SessionStatePatch),
    UpsertTurn(Turn),
    UpsertToolItem(ToolItem),
    UpsertPendingRequest(PendingRequest),
    UpsertRepository(Repository),
    UpsertIntegration(Integration),
}

/// Backend-owned current state. Records are keyed by their exact typed
/// identity; secondary indexes retain source scope for session, thread, turn,
/// tool and agent relationships. Ordered maps make reads deterministic.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct BrokerState {
    sessions: BTreeMap<SourceScopedId, Session>,
    sessions_by_thread: RelationIndex,
    sessions_by_agent: RelationIndex,
    sessions_by_parent_agent: RelationIndex,

    turns: BTreeMap<SourceScopedId, Turn>,
    turns_by_session: RelationIndex,
    turns_by_thread: RelationIndex,
    turns_by_agent: RelationIndex,

    tool_items: BTreeMap<SourceScopedId, ToolItem>,
    tool_items_by_session: RelationIndex,
    tool_items_by_turn: RelationIndex,
    tool_items_by_agent: RelationIndex,

    pending_requests: BTreeMap<SourceScopedId, PendingRequest>,
    requests_by_session: RelationIndex,
    requests_by_thread: RelationIndex,
    requests_by_turn: RelationIndex,
    requests_by_tool_item: RelationIndex,
    requests_by_agent: RelationIndex,

    repositories: BTreeMap<SourceScopedId, Repository>,
    integrations: BTreeMap<OpaqueId, Integration>,
}

impl BrokerState {
    /// Apply one normalized in-memory update. Repeating the exact same
    /// upsert/patch is idempotent. Arrival-order conflict resolution is not
    /// attempted here; CORE-03 owns sequencing and replay.
    pub fn apply(&mut self, update: BrokerUpdate) -> ApplyOutcome {
        match update {
            BrokerUpdate::UpsertSession(value) => self.upsert_session(value),
            BrokerUpdate::PatchSessionState(value) => self.patch_session_state(value),
            BrokerUpdate::UpsertTurn(value) => self.upsert_turn(value),
            BrokerUpdate::UpsertToolItem(value) => self.upsert_tool_item(value),
            BrokerUpdate::UpsertPendingRequest(value) => self.upsert_pending_request(value),
            BrokerUpdate::UpsertRepository(value) => self.upsert_repository(value),
            BrokerUpdate::UpsertIntegration(value) => self.upsert_integration(value),
        }
    }

    pub fn session(&self, identity: &SourceScopedId) -> Option<&Session> {
        self.sessions.get(identity)
    }

    pub fn sessions(&self) -> impl Iterator<Item = &Session> {
        self.sessions.values()
    }

    pub fn sessions_for_thread(&self, thread: &SourceScopedId) -> Vec<&Session> {
        indexed_values(&self.sessions, &self.sessions_by_thread, thread)
    }

    pub fn sessions_for_agent(&self, agent: &SourceScopedId) -> Vec<&Session> {
        indexed_values(&self.sessions, &self.sessions_by_agent, agent)
    }

    pub fn sessions_for_parent_agent(&self, agent: &SourceScopedId) -> Vec<&Session> {
        indexed_values(&self.sessions, &self.sessions_by_parent_agent, agent)
    }

    pub fn turn(&self, identity: &SourceScopedId) -> Option<&Turn> {
        self.turns.get(identity)
    }

    pub fn turns(&self) -> impl Iterator<Item = &Turn> {
        self.turns.values()
    }

    pub fn turns_for_session(&self, session: &SourceScopedId) -> Vec<&Turn> {
        indexed_values(&self.turns, &self.turns_by_session, session)
    }

    pub fn turns_for_thread(&self, thread: &SourceScopedId) -> Vec<&Turn> {
        indexed_values(&self.turns, &self.turns_by_thread, thread)
    }

    pub fn turns_for_agent(&self, agent: &SourceScopedId) -> Vec<&Turn> {
        indexed_values(&self.turns, &self.turns_by_agent, agent)
    }

    pub fn tool_item(&self, identity: &SourceScopedId) -> Option<&ToolItem> {
        self.tool_items.get(identity)
    }

    pub fn tool_items(&self) -> impl Iterator<Item = &ToolItem> {
        self.tool_items.values()
    }

    pub fn tool_items_for_session(&self, session: &SourceScopedId) -> Vec<&ToolItem> {
        indexed_values(&self.tool_items, &self.tool_items_by_session, session)
    }

    pub fn tool_items_for_turn(&self, turn: &SourceScopedId) -> Vec<&ToolItem> {
        indexed_values(&self.tool_items, &self.tool_items_by_turn, turn)
    }

    pub fn tool_items_for_agent(&self, agent: &SourceScopedId) -> Vec<&ToolItem> {
        indexed_values(&self.tool_items, &self.tool_items_by_agent, agent)
    }

    pub fn pending_request(&self, identity: &SourceScopedId) -> Option<&PendingRequest> {
        self.pending_requests.get(identity)
    }

    pub fn pending_requests(&self) -> impl Iterator<Item = &PendingRequest> {
        self.pending_requests.values()
    }

    pub fn requests_for_session(&self, session: &SourceScopedId) -> Vec<&PendingRequest> {
        indexed_values(&self.pending_requests, &self.requests_by_session, session)
    }

    pub fn requests_for_thread(&self, thread: &SourceScopedId) -> Vec<&PendingRequest> {
        indexed_values(&self.pending_requests, &self.requests_by_thread, thread)
    }

    pub fn requests_for_turn(&self, turn: &SourceScopedId) -> Vec<&PendingRequest> {
        indexed_values(&self.pending_requests, &self.requests_by_turn, turn)
    }

    pub fn requests_for_tool_item(&self, tool_item: &SourceScopedId) -> Vec<&PendingRequest> {
        indexed_values(
            &self.pending_requests,
            &self.requests_by_tool_item,
            tool_item,
        )
    }

    pub fn requests_for_agent(&self, agent: &SourceScopedId) -> Vec<&PendingRequest> {
        indexed_values(&self.pending_requests, &self.requests_by_agent, agent)
    }

    pub fn repository(&self, identity: &SourceScopedId) -> Option<&Repository> {
        self.repositories.get(identity)
    }

    pub fn repositories(&self) -> impl Iterator<Item = &Repository> {
        self.repositories.values()
    }

    pub fn integration(&self, identity: &OpaqueId) -> Option<&Integration> {
        self.integrations.get(identity)
    }

    pub fn integrations(&self) -> impl Iterator<Item = &Integration> {
        self.integrations.values()
    }

    fn upsert_session(&mut self, next: Session) -> ApplyOutcome {
        let identity = next.identity.clone();
        if self.sessions.get(&identity) == Some(&next) {
            return ApplyOutcome::Unchanged;
        }
        if let Some(previous) = self.sessions.remove(&identity) {
            self.index_session(&previous, false);
        }
        self.index_session(&next, true);
        self.sessions.insert(identity, next);
        ApplyOutcome::Changed
    }

    fn patch_session_state(&mut self, patch: SessionStatePatch) -> ApplyOutcome {
        if patch.is_empty() && !self.sessions.contains_key(&patch.identity) {
            return ApplyOutcome::Unchanged;
        }
        let mut session = self
            .sessions
            .get(&patch.identity)
            .cloned()
            .unwrap_or_else(|| empty_session(patch.identity.clone()));
        patch.apply_to(&mut session);
        self.upsert_session(session)
    }

    fn upsert_turn(&mut self, next: Turn) -> ApplyOutcome {
        let identity = next.identity.clone();
        if self.turns.get(&identity) == Some(&next) {
            return ApplyOutcome::Unchanged;
        }
        if let Some(previous) = self.turns.remove(&identity) {
            self.index_turn(&previous, false);
        }
        self.index_turn(&next, true);
        self.turns.insert(identity, next);
        ApplyOutcome::Changed
    }

    fn upsert_tool_item(&mut self, next: ToolItem) -> ApplyOutcome {
        let identity = next.identity.clone();
        if self.tool_items.get(&identity) == Some(&next) {
            return ApplyOutcome::Unchanged;
        }
        if let Some(previous) = self.tool_items.remove(&identity) {
            self.index_tool_item(&previous, false);
        }
        self.index_tool_item(&next, true);
        self.tool_items.insert(identity, next);
        ApplyOutcome::Changed
    }

    fn upsert_pending_request(&mut self, next: PendingRequest) -> ApplyOutcome {
        let identity = next.identity.clone();
        if self.pending_requests.get(&identity) == Some(&next) {
            return ApplyOutcome::Unchanged;
        }
        if let Some(previous) = self.pending_requests.remove(&identity) {
            self.index_request(&previous, false);
        }
        self.index_request(&next, true);
        self.pending_requests.insert(identity, next);
        ApplyOutcome::Changed
    }

    fn upsert_repository(&mut self, next: Repository) -> ApplyOutcome {
        let identity = next.identity.clone();
        if self.repositories.get(&identity) == Some(&next) {
            return ApplyOutcome::Unchanged;
        }
        self.repositories.insert(identity, next);
        ApplyOutcome::Changed
    }

    fn upsert_integration(&mut self, next: Integration) -> ApplyOutcome {
        let identity = next.identity.clone();
        if self.integrations.get(&identity) == Some(&next) {
            return ApplyOutcome::Unchanged;
        }
        self.integrations.insert(identity, next);
        ApplyOutcome::Changed
    }

    fn index_session(&mut self, session: &Session, insert: bool) {
        let source = session.identity.source;
        change_optional_index(
            &mut self.sessions_by_thread,
            source,
            session.thread_id.as_ref(),
            &session.identity,
            insert,
        );
        change_optional_index(
            &mut self.sessions_by_agent,
            source,
            session.agent_id.as_ref(),
            &session.identity,
            insert,
        );
        change_optional_index(
            &mut self.sessions_by_parent_agent,
            source,
            session.parent_agent_id.as_ref(),
            &session.identity,
            insert,
        );
    }

    fn index_turn(&mut self, turn: &Turn, insert: bool) {
        change_index(
            &mut self.turns_by_session,
            Some(turn.session_id.clone()),
            &turn.identity,
            insert,
        );
        let source = turn.identity.source;
        change_optional_index(
            &mut self.turns_by_thread,
            source,
            turn.thread_id.as_ref(),
            &turn.identity,
            insert,
        );
        change_optional_index(
            &mut self.turns_by_agent,
            source,
            turn.agent_id.as_ref(),
            &turn.identity,
            insert,
        );
    }

    fn index_tool_item(&mut self, item: &ToolItem, insert: bool) {
        change_index(
            &mut self.tool_items_by_session,
            Some(item.session_id.clone()),
            &item.identity,
            insert,
        );
        change_index(
            &mut self.tool_items_by_turn,
            item.turn_id.clone(),
            &item.identity,
            insert,
        );
        change_optional_index(
            &mut self.tool_items_by_agent,
            item.identity.source,
            item.agent_id.as_ref(),
            &item.identity,
            insert,
        );
    }

    fn index_request(&mut self, request: &PendingRequest, insert: bool) {
        change_index(
            &mut self.requests_by_session,
            request.session_id.clone(),
            &request.identity,
            insert,
        );
        // CORE-01 stores these related values as opaque IDs. Scope them to
        // their linked session when present; detached requests stay in their
        // producer's namespace. This is an internal index rule, not a source
        // event interpretation.
        let source = request
            .session_id
            .as_ref()
            .map_or(request.identity.source, |id| id.source);
        change_optional_index(
            &mut self.requests_by_thread,
            source,
            request.thread_id.as_ref(),
            &request.identity,
            insert,
        );
        change_optional_index(
            &mut self.requests_by_turn,
            source,
            request.turn_id.as_ref(),
            &request.identity,
            insert,
        );
        change_optional_index(
            &mut self.requests_by_tool_item,
            source,
            request.tool_item_id.as_ref(),
            &request.identity,
            insert,
        );
        change_optional_index(
            &mut self.requests_by_agent,
            source,
            request.agent_id.as_ref(),
            &request.identity,
            insert,
        );
    }
}

fn empty_session(identity: SourceScopedId) -> Session {
    Session {
        schema_version: SchemaVersion::current(),
        identity,
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

fn scoped(source: SourceKind, id: &OpaqueId) -> SourceScopedId {
    SourceScopedId::new(source, id.clone())
}

fn change_optional_index(
    index: &mut RelationIndex,
    source: SourceKind,
    related_id: Option<&OpaqueId>,
    identity: &SourceScopedId,
    insert: bool,
) {
    change_index(
        index,
        related_id.map(|id| scoped(source, id)),
        identity,
        insert,
    );
}

fn change_index(
    index: &mut RelationIndex,
    key: Option<SourceScopedId>,
    identity: &SourceScopedId,
    insert: bool,
) {
    let Some(key) = key else {
        return;
    };
    if insert {
        index.entry(key).or_default().insert(identity.clone());
    } else if let Some(identities) = index.get_mut(&key) {
        identities.remove(identity);
        if identities.is_empty() {
            index.remove(&key);
        }
    }
}

fn indexed_values<'a, T>(
    records: &'a BTreeMap<SourceScopedId, T>,
    index: &RelationIndex,
    key: &SourceScopedId,
) -> Vec<&'a T> {
    index
        .get(key)
        .into_iter()
        .flat_map(BTreeSet::iter)
        .filter_map(|identity| records.get(identity))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::broker::types::{
        ConfigurationState, IntegrationProvider, RequestKind, RequestLifecycle, SessionActivity,
        SessionLifecycle, ToolItemState, TurnLifecycle, TurnOutcome,
    };

    fn id(value: &str) -> OpaqueId {
        OpaqueId::new(value).unwrap()
    }

    fn scoped_id(source: SourceKind, value: &str) -> SourceScopedId {
        SourceScopedId::new(source, id(value))
    }

    fn session(
        source: SourceKind,
        identity: &str,
        thread: Option<&str>,
        agent: Option<&str>,
    ) -> Session {
        Session {
            schema_version: SchemaVersion::current(),
            identity: scoped_id(source, identity),
            thread_id: thread.map(id),
            agent_id: agent.map(id),
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

    fn turn(source: SourceKind, identity: &str, session_id: &str) -> Turn {
        Turn {
            schema_version: SchemaVersion::current(),
            identity: scoped_id(source, identity),
            session_id: scoped_id(source, session_id),
            thread_id: Some(id("thread-1")),
            agent_id: Some(id("agent-1")),
            lifecycle: TurnLifecycle::Active,
            outcome: TurnOutcome::Unknown,
            started_at_unix_ms: None,
            ended_at_unix_ms: None,
            summary: None,
            plan: None,
        }
    }

    fn tool_item(source: SourceKind, identity: &str, session_id: &str, turn_id: &str) -> ToolItem {
        ToolItem {
            schema_version: SchemaVersion::current(),
            identity: scoped_id(source, identity),
            session_id: scoped_id(source, session_id),
            turn_id: Some(scoped_id(source, turn_id)),
            agent_id: Some(id("agent-1")),
            display_name: Some("shell".to_string()),
            sanitized_target: None,
            state: ToolItemState::Active,
            started_at_unix_ms: None,
            ended_at_unix_ms: None,
            exit_code: None,
        }
    }

    fn request(source: SourceKind, identity: &str, session_id: &str) -> PendingRequest {
        PendingRequest {
            schema_version: SchemaVersion::current(),
            identity: scoped_id(source, identity),
            session_id: Some(scoped_id(source, session_id)),
            thread_id: Some(id("thread-1")),
            turn_id: Some(id("turn-1")),
            tool_item_id: Some(id("tool-1")),
            agent_id: Some(id("agent-1")),
            kind: RequestKind::Approval,
            requested_capability: None,
            lifecycle: RequestLifecycle::Pending,
            created_at_unix_ms: None,
            deadline_unix_ms: None,
        }
    }

    #[test]
    fn identical_ids_from_different_sources_have_independent_state_and_indexes() {
        let mut state = BrokerState::default();
        let mut codex = session(
            SourceKind::CodexCliObserver,
            "same",
            Some("thread"),
            Some("agent"),
        );
        codex.activity = SessionActivity::Working;
        let mut github = session(SourceKind::GitHub, "same", Some("thread"), Some("agent"));
        github.activity = SessionActivity::Idle;

        assert_eq!(
            state.apply(BrokerUpdate::UpsertSession(codex)),
            ApplyOutcome::Changed
        );
        assert_eq!(
            state.apply(BrokerUpdate::UpsertSession(github)),
            ApplyOutcome::Changed
        );
        assert_eq!(state.sessions().count(), 2);
        assert_eq!(
            state
                .session(&scoped_id(SourceKind::CodexCliObserver, "same"))
                .unwrap()
                .activity,
            SessionActivity::Working
        );
        assert_eq!(
            state
                .session(&scoped_id(SourceKind::GitHub, "same"))
                .unwrap()
                .activity,
            SessionActivity::Idle
        );
        assert_eq!(
            state
                .sessions_for_thread(&scoped_id(SourceKind::CodexCliObserver, "thread"))
                .len(),
            1
        );
        assert_eq!(
            state
                .sessions_for_agent(&scoped_id(SourceKind::GitHub, "agent"))
                .len(),
            1
        );
    }

    #[test]
    fn child_records_can_arrive_before_their_session_and_remain_queryable() {
        let mut state = BrokerState::default();
        let turn = turn(SourceKind::CodexCliObserver, "turn-1", "session-1");
        let item = tool_item(
            SourceKind::CodexCliObserver,
            "tool-1",
            "session-1",
            "turn-1",
        );
        let pending = request(SourceKind::CodexCliObserver, "request-1", "session-1");

        assert_eq!(
            state.apply(BrokerUpdate::UpsertTurn(turn.clone())),
            ApplyOutcome::Changed
        );
        assert_eq!(
            state.apply(BrokerUpdate::UpsertToolItem(item.clone())),
            ApplyOutcome::Changed
        );
        assert_eq!(
            state.apply(BrokerUpdate::UpsertPendingRequest(pending.clone())),
            ApplyOutcome::Changed
        );
        let session_key = scoped_id(SourceKind::CodexCliObserver, "session-1");
        let thread_key = scoped_id(SourceKind::CodexCliObserver, "thread-1");
        let agent_key = scoped_id(SourceKind::CodexCliObserver, "agent-1");
        let turn_key = scoped_id(SourceKind::CodexCliObserver, "turn-1");
        let tool_key = scoped_id(SourceKind::CodexCliObserver, "tool-1");

        assert_eq!(state.turns_for_session(&session_key), vec![&turn]);
        assert_eq!(state.turns_for_thread(&thread_key), vec![&turn]);
        assert_eq!(state.turns_for_agent(&agent_key), vec![&turn]);
        assert_eq!(state.tool_items_for_turn(&turn_key), vec![&item]);
        assert_eq!(state.requests_for_session(&session_key), vec![&pending]);
        assert_eq!(state.requests_for_thread(&thread_key), vec![&pending]);
        assert_eq!(state.requests_for_turn(&turn_key), vec![&pending]);
        assert_eq!(state.requests_for_tool_item(&tool_key), vec![&pending]);
        assert_eq!(state.requests_for_agent(&agent_key), vec![&pending]);

        assert_eq!(
            state.apply(BrokerUpdate::UpsertTurn(turn)),
            ApplyOutcome::Unchanged
        );
        assert_eq!(
            state.apply(BrokerUpdate::UpsertToolItem(item)),
            ApplyOutcome::Unchanged
        );
        assert_eq!(
            state.apply(BrokerUpdate::UpsertPendingRequest(pending)),
            ApplyOutcome::Unchanged
        );
        assert_eq!(
            state.apply(BrokerUpdate::UpsertSession(session(
                SourceKind::CodexCliObserver,
                "session-1",
                Some("thread-1"),
                Some("agent-1"),
            ))),
            ApplyOutcome::Changed
        );
        assert_eq!(state.turns_for_session(&session_key).len(), 1);
        assert_eq!(state.tool_items_for_session(&session_key).len(), 1);
        assert_eq!(state.requests_for_session(&session_key).len(), 1);
    }

    #[test]
    fn replacing_session_aliases_removes_stale_thread_and_agent_indexes() {
        let mut state = BrokerState::default();
        let old = session(
            SourceKind::CodexCliObserver,
            "session-1",
            Some("old-thread"),
            Some("old-agent"),
        );
        state.apply(BrokerUpdate::UpsertSession(old));

        let next = session(
            SourceKind::CodexCliObserver,
            "session-1",
            Some("new-thread"),
            Some("new-agent"),
        );
        assert_eq!(
            state.apply(BrokerUpdate::UpsertSession(next)),
            ApplyOutcome::Changed
        );
        assert!(state
            .sessions_for_thread(&scoped_id(SourceKind::CodexCliObserver, "old-thread"))
            .is_empty());
        assert!(state
            .sessions_for_agent(&scoped_id(SourceKind::CodexCliObserver, "old-agent"))
            .is_empty());
        assert_eq!(
            state
                .sessions_for_thread(&scoped_id(SourceKind::CodexCliObserver, "new-thread"))
                .len(),
            1
        );
        assert_eq!(
            state
                .sessions_for_agent(&scoped_id(SourceKind::CodexCliObserver, "new-agent"))
                .len(),
            1
        );
    }

    #[test]
    fn replacing_child_relationships_removes_every_old_index_entry() {
        let mut state = BrokerState::default();
        let source = SourceKind::CodexCliObserver;
        let old_session = scoped_id(source, "session-1");
        let new_session = scoped_id(source, "session-2");
        let old_thread = scoped_id(source, "thread-1");
        let new_thread = scoped_id(source, "thread-2");
        let old_agent = scoped_id(source, "agent-1");
        let new_agent = scoped_id(source, "agent-2");
        let old_turn = scoped_id(source, "turn-1");
        let new_turn = scoped_id(source, "turn-2");
        let old_tool = scoped_id(source, "tool-1");
        let new_tool = scoped_id(source, "tool-2");

        let first_turn = turn(source, "turn-record", "session-1");
        state.apply(BrokerUpdate::UpsertTurn(first_turn.clone()));
        let mut next_turn = first_turn;
        next_turn.session_id = new_session.clone();
        next_turn.thread_id = Some(id("thread-2"));
        next_turn.agent_id = Some(id("agent-2"));
        assert_eq!(
            state.apply(BrokerUpdate::UpsertTurn(next_turn)),
            ApplyOutcome::Changed
        );
        assert!(state.turns_for_session(&old_session).is_empty());
        assert!(state.turns_for_thread(&old_thread).is_empty());
        assert!(state.turns_for_agent(&old_agent).is_empty());
        assert_eq!(state.turns_for_session(&new_session).len(), 1);
        assert_eq!(state.turns_for_thread(&new_thread).len(), 1);
        assert_eq!(state.turns_for_agent(&new_agent).len(), 1);

        let first_item = tool_item(source, "tool-record", "session-1", "turn-1");
        state.apply(BrokerUpdate::UpsertToolItem(first_item.clone()));
        let mut next_item = first_item;
        next_item.session_id = new_session.clone();
        next_item.turn_id = Some(new_turn.clone());
        next_item.agent_id = Some(id("agent-2"));
        assert_eq!(
            state.apply(BrokerUpdate::UpsertToolItem(next_item)),
            ApplyOutcome::Changed
        );
        assert!(state.tool_items_for_session(&old_session).is_empty());
        assert!(state.tool_items_for_turn(&old_turn).is_empty());
        assert!(state.tool_items_for_agent(&old_agent).is_empty());
        assert_eq!(state.tool_items_for_session(&new_session).len(), 1);
        assert_eq!(state.tool_items_for_turn(&new_turn).len(), 1);
        assert_eq!(state.tool_items_for_agent(&new_agent).len(), 1);

        let first_request = request(source, "request-record", "session-1");
        state.apply(BrokerUpdate::UpsertPendingRequest(first_request.clone()));
        let mut next_request = first_request;
        next_request.session_id = Some(new_session.clone());
        next_request.thread_id = Some(id("thread-2"));
        next_request.turn_id = Some(id("turn-2"));
        next_request.tool_item_id = Some(id("tool-2"));
        next_request.agent_id = Some(id("agent-2"));
        assert_eq!(
            state.apply(BrokerUpdate::UpsertPendingRequest(next_request)),
            ApplyOutcome::Changed
        );
        assert!(state.requests_for_session(&old_session).is_empty());
        assert!(state.requests_for_thread(&old_thread).is_empty());
        assert!(state.requests_for_turn(&old_turn).is_empty());
        assert!(state.requests_for_tool_item(&old_tool).is_empty());
        assert!(state.requests_for_agent(&old_agent).is_empty());
        assert_eq!(state.requests_for_session(&new_session).len(), 1);
        assert_eq!(state.requests_for_thread(&new_thread).len(), 1);
        assert_eq!(state.requests_for_turn(&new_turn).len(), 1);
        assert_eq!(state.requests_for_tool_item(&new_tool).len(), 1);
        assert_eq!(state.requests_for_agent(&new_agent).len(), 1);
    }

    #[test]
    fn session_state_patch_changes_only_the_named_axes() {
        let mut state = BrokerState::default();
        let mut value = session(SourceKind::CodexCliObserver, "session-1", None, None);
        value.lifecycle = SessionLifecycle::Active;
        value.activity = SessionActivity::Idle;
        value.waiting = WaitingState::Approval;
        value.connection = ConnectionHealth::Degraded;
        state.apply(BrokerUpdate::UpsertSession(value));

        let patch = SessionStatePatch {
            identity: scoped_id(SourceKind::CodexCliObserver, "session-1"),
            lifecycle: None,
            activity: Some(SessionActivity::Working),
            waiting: None,
            connection: None,
        };
        assert_eq!(
            state.apply(BrokerUpdate::PatchSessionState(patch.clone())),
            ApplyOutcome::Changed
        );
        assert_eq!(
            state.apply(BrokerUpdate::PatchSessionState(patch)),
            ApplyOutcome::Unchanged
        );
        let actual = state
            .session(&scoped_id(SourceKind::CodexCliObserver, "session-1"))
            .unwrap();
        assert_eq!(actual.lifecycle, SessionLifecycle::Active);
        assert_eq!(actual.activity, SessionActivity::Working);
        assert_eq!(actual.waiting, WaitingState::Approval);
        assert_eq!(actual.connection, ConnectionHealth::Degraded);

        assert_eq!(
            state.apply(BrokerUpdate::PatchSessionState(SessionStatePatch {
                identity: scoped_id(SourceKind::CodexCliObserver, "session-1"),
                lifecycle: None,
                activity: None,
                waiting: Some(WaitingState::Unknown),
                connection: None,
            })),
            ApplyOutcome::Changed
        );
        let actual = state
            .session(&scoped_id(SourceKind::CodexCliObserver, "session-1"))
            .unwrap();
        assert_eq!(actual.lifecycle, SessionLifecycle::Active);
        assert_eq!(actual.activity, SessionActivity::Working);
        assert_eq!(actual.waiting, WaitingState::Unknown);
        assert_eq!(actual.connection, ConnectionHealth::Degraded);
    }

    #[test]
    fn request_relationship_ids_use_the_linked_session_source() {
        let mut state = BrokerState::default();
        let mut value = request(SourceKind::Windows, "request-1", "session-1");
        value.session_id = Some(scoped_id(SourceKind::CodexCliObserver, "session-1"));
        state.apply(BrokerUpdate::UpsertPendingRequest(value));

        assert_eq!(
            state
                .requests_for_thread(&scoped_id(SourceKind::CodexCliObserver, "thread-1"))
                .len(),
            1
        );
        assert!(state
            .requests_for_thread(&scoped_id(SourceKind::Windows, "thread-1"))
            .is_empty());
    }

    #[test]
    fn state_patch_for_a_new_session_creates_unknown_unmodified_axes() {
        let mut state = BrokerState::default();
        let identity = scoped_id(SourceKind::CodexCliObserver, "session-1");
        let patch = SessionStatePatch {
            identity: identity.clone(),
            lifecycle: Some(SessionLifecycle::Active),
            activity: None,
            waiting: None,
            connection: None,
        };
        assert_eq!(
            state.apply(BrokerUpdate::PatchSessionState(patch)),
            ApplyOutcome::Changed
        );
        let actual = state.session(&identity).unwrap();
        assert_eq!(actual.lifecycle, SessionLifecycle::Active);
        assert_eq!(actual.activity, SessionActivity::Unknown);
        assert_eq!(actual.waiting, WaitingState::Unknown);
        assert_eq!(actual.connection, ConnectionHealth::Unknown);
    }

    #[test]
    fn repository_and_integration_records_are_reduced_by_their_identities() {
        let mut state = BrokerState::default();
        let mut repository = Repository {
            schema_version: SchemaVersion::current(),
            identity: scoped_id(SourceKind::LocalGit, "repo-1"),
            host: Some("github.com".to_string()),
            owner: Some("owner".to_string()),
            name: Some("repo".to_string()),
            worktree_id: None,
            branch: None,
            head_sha: None,
            observed_at_unix_ms: None,
        };
        assert_eq!(
            state.apply(BrokerUpdate::UpsertRepository(repository.clone())),
            ApplyOutcome::Changed
        );
        assert_eq!(
            state.apply(BrokerUpdate::UpsertRepository(repository.clone())),
            ApplyOutcome::Unchanged
        );
        repository.branch = Some("main".to_string());
        assert_eq!(
            state.apply(BrokerUpdate::UpsertRepository(repository)),
            ApplyOutcome::Changed
        );
        assert_eq!(
            state
                .repository(&scoped_id(SourceKind::LocalGit, "repo-1"))
                .unwrap()
                .branch
                .as_deref(),
            Some("main")
        );

        let mut integration = Integration {
            schema_version: SchemaVersion::current(),
            identity: id("github-account"),
            provider: IntegrationProvider::GitHub,
            configuration: ConfigurationState::Configured,
            connection: ConnectionHealth::Healthy,
            last_success_at_unix_ms: None,
            data_revision: None,
            retry_at_unix_ms: None,
            last_error: None,
            unread_event_ids: Vec::new(),
        };
        assert_eq!(
            state.apply(BrokerUpdate::UpsertIntegration(integration.clone())),
            ApplyOutcome::Changed
        );
        integration.connection = ConnectionHealth::Degraded;
        assert_eq!(
            state.apply(BrokerUpdate::UpsertIntegration(integration)),
            ApplyOutcome::Changed
        );
        assert_eq!(
            state.integration(&id("github-account")).unwrap().connection,
            ConnectionHealth::Degraded
        );
    }
}
