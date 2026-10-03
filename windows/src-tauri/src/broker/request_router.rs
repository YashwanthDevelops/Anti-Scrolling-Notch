//! Fail-closed routing contract for normalized backend requests.
//!
//! This router is not connected to a Codex hook producer, Tauri command, UI,
//! or reply transport. It accepts only existing PendingRequest records and
//! reserves one process-local reply attempt after exact presentation and
//! capability checks. A transport result does not by itself resolve the
//! source request.

use std::collections::BTreeMap;
use std::fmt;

use super::types::{OpaqueId, PendingRequest, RequestKind, RequestLifecycle, SourceScopedId};
use crate::capabilities::{self, CapabilityRegistry, PERMISSION_DECISION_CAPABILITY};

pub const MAX_TRACKED_REQUESTS: usize = 64;
pub const MAX_CONCURRENT_REPLY_ATTEMPTS: usize = 4;

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct RequestRouteKey {
    request_id: SourceScopedId,
    session_id: Option<SourceScopedId>,
    thread_id: Option<OpaqueId>,
    turn_id: Option<OpaqueId>,
    tool_item_id: Option<OpaqueId>,
    agent_id: Option<OpaqueId>,
}

impl RequestRouteKey {
    pub fn request_id(&self) -> &SourceScopedId {
        &self.request_id
    }

    pub fn session_id(&self) -> Option<&SourceScopedId> {
        self.session_id.as_ref()
    }

    pub fn thread_id(&self) -> Option<&OpaqueId> {
        self.thread_id.as_ref()
    }

    pub fn turn_id(&self) -> Option<&OpaqueId> {
        self.turn_id.as_ref()
    }

    pub fn tool_item_id(&self) -> Option<&OpaqueId> {
        self.tool_item_id.as_ref()
    }

    pub fn agent_id(&self) -> Option<&OpaqueId> {
        self.agent_id.as_ref()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct RequestSignature {
    key: RequestRouteKey,
    kind: RequestKind,
    requested_capability: Option<OpaqueId>,
    created_at_unix_ms: Option<u64>,
    deadline_unix_ms: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum PresentationState {
    NotIssued,
    AwaitingAcknowledgement(u64),
    Acknowledged(u64),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum AttemptState {
    Available,
    InFlight(u64),
    Completed(DeliveryOutcome),
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct RequestEntry {
    signature: RequestSignature,
    presentation: PresentationState,
    attempt: AttemptState,
}

/// Opaque token proving that a particular request was presented and that the
/// UI acknowledged the exact presentation instance.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PresentationToken {
    key: RequestRouteKey,
    generation: u64,
}

impl PresentationToken {
    pub fn request(&self) -> &RequestRouteKey {
        &self.key
    }
}

/// Single-use authorization to invoke a future adapter outside the router's
/// serialized state owner. This value is deliberately not Clone.
#[derive(Debug, Eq, PartialEq)]
pub struct ReplyAttempt {
    key: RequestRouteKey,
    token: u64,
    decision: PermissionDecision,
}

impl ReplyAttempt {
    pub fn request(&self) -> &RequestRouteKey {
        &self.key
    }

    pub const fn decision(&self) -> PermissionDecision {
        self.decision
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PermissionDecision {
    Allow,
    Deny,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DeliveryOutcome {
    /// The adapter reports that it accepted the reply. Source resolution still
    /// requires an authoritative source update.
    AcceptedByAdapter,
    /// The adapter proves that no reply was sent. This router still does not
    /// retry; retry policy requires a separately verified source contract.
    RejectedBeforeSend,
    /// Delivery may have reached the source. Never replay this attempt.
    Uncertain,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReplyCompletion {
    pub request: RequestRouteKey,
    pub outcome: DeliveryOutcome,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Registration {
    Inserted,
    AlreadyRegistered,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TerminalCleanup {
    Removed,
    NotTracked,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RouteError {
    InvalidSourceScope,
    UnsupportedLifecycle,
    TerminalUpdateRequired,
    MissingDeadline,
    DeadlineElapsed,
    RequestCapacityReached,
    UnknownRequest,
    ConflictingRequestIdentity,
    AlreadyAcknowledged,
    PresentationNotAcknowledged,
    StalePresentation,
    UnsupportedRequestKind,
    CapabilityUnavailable,
    ReplyConcurrencyLimit,
    ReplyAlreadyClaimed,
    AttemptIdExhausted,
    StaleAttempt,
}

impl fmt::Display for RouteError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::InvalidSourceScope => "request and session source identities do not match",
            Self::UnsupportedLifecycle => "request is not pending or presented",
            Self::TerminalUpdateRequired => "request update is not resolved, expired or cancelled",
            Self::MissingDeadline => "request has no source-supplied deadline",
            Self::DeadlineElapsed => "request deadline has elapsed",
            Self::RequestCapacityReached => "request router capacity is full",
            Self::UnknownRequest => "request is not registered",
            Self::ConflictingRequestIdentity => "request identity conflicts with an existing route",
            Self::AlreadyAcknowledged => "presentation acknowledgement was already accepted",
            Self::PresentationNotAcknowledged => "request was not acknowledged as visible",
            Self::StalePresentation => "presentation token is stale or belongs to another request",
            Self::UnsupportedRequestKind => "request kind has no verified reply contract",
            Self::CapabilityUnavailable => "Codex reply capability is unavailable or stale",
            Self::ReplyConcurrencyLimit => "reply concurrency limit is full",
            Self::ReplyAlreadyClaimed => "request already has a reply attempt",
            Self::AttemptIdExhausted => "reply attempt identifiers are exhausted",
            Self::StaleAttempt => "reply attempt is stale or already completed",
        })
    }
}

impl std::error::Error for RouteError {}

/// Process-local bounded routing state. Entries are not silently evicted:
/// once its fixed capacity is reached, registration fails closed until an
/// authoritative terminal source update releases a tracked route.
#[derive(Debug)]
pub struct RequestRouter {
    entries: BTreeMap<SourceScopedId, RequestEntry>,
    next_token: u64,
}

impl Default for RequestRouter {
    fn default() -> Self {
        Self::new()
    }
}

impl RequestRouter {
    pub fn new() -> Self {
        Self {
            entries: BTreeMap::new(),
            next_token: 1,
        }
    }

    pub fn tracked_requests(&self) -> usize {
        self.entries.len()
    }

    pub fn in_flight_replies(&self) -> usize {
        self.entries
            .values()
            .filter(|entry| matches!(entry.attempt, AttemptState::InFlight(_)))
            .count()
    }

    /// Register an exact source request once. Replaying identical registration
    /// is idempotent and never resets presentation or attempt state.
    pub fn register(
        &mut self,
        request: &PendingRequest,
        now_unix_ms: u64,
    ) -> Result<Registration, RouteError> {
        let signature = signature(request, now_unix_ms)?;
        let id = signature.key.request_id.clone();
        if let Some(existing) = self.entries.get(&id) {
            return if existing.signature == signature {
                Ok(Registration::AlreadyRegistered)
            } else {
                Err(RouteError::ConflictingRequestIdentity)
            };
        }
        if self.entries.len() >= MAX_TRACKED_REQUESTS {
            return Err(RouteError::RequestCapacityReached);
        }
        self.entries.insert(
            id,
            RequestEntry {
                signature,
                presentation: PresentationState::NotIssued,
                attempt: AttemptState::Available,
            },
        );
        Ok(Registration::Inserted)
    }

    /// Release a route only after an authoritative source update reports a
    /// terminal lifecycle. Exact request metadata must still match the route,
    /// even when its deadline has elapsed. This handles resolution in another
    /// client without treating a local adapter result as source resolution.
    pub fn clear_terminal_update(
        &mut self,
        request: &PendingRequest,
    ) -> Result<TerminalCleanup, RouteError> {
        if !matches!(
            request.lifecycle,
            RequestLifecycle::Resolved | RequestLifecycle::Expired | RequestLifecycle::Cancelled
        ) {
            return Err(RouteError::TerminalUpdateRequired);
        }

        let id = request.identity.clone();
        let Some(entry) = self.entries.get(&id) else {
            return Ok(TerminalCleanup::NotTracked);
        };
        let current = source_signature(request)?;
        ensure_same_signature(entry, &current)?;

        self.entries.remove(&id);
        Ok(TerminalCleanup::Removed)
    }

    /// Begin a presentation attempt. A later acknowledgement must carry the
    /// returned token; retries invalidate older outstanding tokens.
    pub fn begin_presentation(
        &mut self,
        request: &PendingRequest,
        now_unix_ms: u64,
    ) -> Result<PresentationToken, RouteError> {
        let current = signature(request, now_unix_ms)?;
        let id = current.key.request_id.clone();
        let entry = self.entries.get(&id).ok_or(RouteError::UnknownRequest)?;
        ensure_same_signature(entry, &current)?;
        if entry.attempt != AttemptState::Available {
            return Err(RouteError::ReplyAlreadyClaimed);
        }
        let generation = self.next_token()?;
        let token = PresentationToken {
            key: current.key,
            generation,
        };
        self.entries
            .get_mut(&id)
            .expect("entry was checked above")
            .presentation = PresentationState::AwaitingAcknowledgement(generation);
        Ok(token)
    }

    /// Accept an acknowledgement only for the current presentation token and
    /// exact request. Duplicate acknowledgements do not extend its deadline.
    pub fn acknowledge_presentation(
        &mut self,
        request: &PendingRequest,
        token: &PresentationToken,
        now_unix_ms: u64,
    ) -> Result<(), RouteError> {
        let current = signature(request, now_unix_ms)?;
        if current.key != token.key {
            return Err(RouteError::StalePresentation);
        }
        let id = current.key.request_id.clone();
        let entry = self
            .entries
            .get_mut(&id)
            .ok_or(RouteError::UnknownRequest)?;
        ensure_same_signature(entry, &current)?;
        match entry.presentation {
            PresentationState::AwaitingAcknowledgement(generation)
                if generation == token.generation =>
            {
                entry.presentation = PresentationState::Acknowledged(generation);
                Ok(())
            }
            PresentationState::Acknowledged(generation) if generation == token.generation => {
                Err(RouteError::AlreadyAcknowledged)
            }
            _ => Err(RouteError::StalePresentation),
        }
    }

    /// Reserve exactly one permission-decision effect after backend capability,
    /// identity, acknowledgement, deadline, and concurrency checks. The
    /// caller must run any adapter IO after this method returns.
    pub fn claim_reply(
        &mut self,
        registry: &CapabilityRegistry,
        expected_capability_generation: &str,
        request: &PendingRequest,
        presentation: &PresentationToken,
        decision: PermissionDecision,
        now_unix_ms: u64,
    ) -> Result<ReplyAttempt, RouteError> {
        let current = signature(request, now_unix_ms)?;
        if current.key != presentation.key {
            return Err(RouteError::StalePresentation);
        }
        if current.kind != RequestKind::Approval {
            return Err(RouteError::UnsupportedRequestKind);
        }

        capabilities::execute(
            registry,
            expected_capability_generation,
            PERMISSION_DECISION_CAPABILITY,
            || Ok(()),
        )
        .map_err(|_| RouteError::CapabilityUnavailable)?;
        self.claim_authorized(current, presentation, decision)
    }

    /// Consume a reservation once. `AcceptedByAdapter` describes transport
    /// acceptance only; the source must still authoritatively resolve the
    /// request. Failures and uncertain delivery are terminal for this route.
    pub fn complete_reply(
        &mut self,
        attempt: ReplyAttempt,
        outcome: DeliveryOutcome,
    ) -> Result<ReplyCompletion, RouteError> {
        let id = attempt.key.request_id.clone();
        let entry = self
            .entries
            .get_mut(&id)
            .ok_or(RouteError::UnknownRequest)?;
        if entry.signature.key != attempt.key
            || entry.attempt != AttemptState::InFlight(attempt.token)
        {
            return Err(RouteError::StaleAttempt);
        }
        entry.attempt = AttemptState::Completed(outcome);
        Ok(ReplyCompletion {
            request: attempt.key,
            outcome,
        })
    }

    fn claim_authorized(
        &mut self,
        current: RequestSignature,
        presentation: &PresentationToken,
        decision: PermissionDecision,
    ) -> Result<ReplyAttempt, RouteError> {
        let id = current.key.request_id.clone();
        let entry = self.entries.get(&id).ok_or(RouteError::UnknownRequest)?;
        ensure_same_signature(entry, &current)?;
        if entry.presentation != PresentationState::Acknowledged(presentation.generation) {
            return match entry.presentation {
                PresentationState::AwaitingAcknowledgement(_) | PresentationState::NotIssued => {
                    Err(RouteError::PresentationNotAcknowledged)
                }
                PresentationState::Acknowledged(_) => Err(RouteError::StalePresentation),
            };
        }
        if entry.attempt != AttemptState::Available {
            return Err(RouteError::ReplyAlreadyClaimed);
        }
        if self.in_flight_replies() >= MAX_CONCURRENT_REPLY_ATTEMPTS {
            return Err(RouteError::ReplyConcurrencyLimit);
        }
        let token = self.next_token()?;
        self.entries
            .get_mut(&id)
            .expect("entry was checked above")
            .attempt = AttemptState::InFlight(token);
        Ok(ReplyAttempt {
            key: current.key,
            token,
            decision,
        })
    }

    fn next_token(&mut self) -> Result<u64, RouteError> {
        let token = self.next_token;
        self.next_token = self
            .next_token
            .checked_add(1)
            .ok_or(RouteError::AttemptIdExhausted)?;
        Ok(token)
    }
}

fn signature(request: &PendingRequest, now_unix_ms: u64) -> Result<RequestSignature, RouteError> {
    if !matches!(
        request.lifecycle,
        RequestLifecycle::Pending | RequestLifecycle::Presented
    ) {
        return Err(RouteError::UnsupportedLifecycle);
    }
    let current = source_signature(request)?;
    if now_unix_ms >= current.deadline_unix_ms {
        return Err(RouteError::DeadlineElapsed);
    }
    Ok(current)
}

fn source_signature(request: &PendingRequest) -> Result<RequestSignature, RouteError> {
    if request
        .session_id
        .as_ref()
        .is_some_and(|session| session.source != request.identity.source)
    {
        return Err(RouteError::InvalidSourceScope);
    }
    let deadline_unix_ms = request
        .deadline_unix_ms
        .ok_or(RouteError::MissingDeadline)?;
    Ok(RequestSignature {
        key: RequestRouteKey {
            request_id: request.identity.clone(),
            session_id: request.session_id.clone(),
            thread_id: request.thread_id.clone(),
            turn_id: request.turn_id.clone(),
            tool_item_id: request.tool_item_id.clone(),
            agent_id: request.agent_id.clone(),
        },
        kind: request.kind,
        requested_capability: request.requested_capability.clone(),
        created_at_unix_ms: request.created_at_unix_ms,
        deadline_unix_ms,
    })
}

fn ensure_same_signature(
    entry: &RequestEntry,
    current: &RequestSignature,
) -> Result<(), RouteError> {
    if entry.signature == *current {
        Ok(())
    } else {
        Err(RouteError::ConflictingRequestIdentity)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::broker::reducer::BrokerUpdate;
    use crate::broker::service::BrokerService;
    use crate::broker::stream::BrokerSyncResponse;
    use crate::broker::types::{SchemaVersion, SourceKind};
    use crate::capabilities::{CapabilityEntry, REGISTRY_SCHEMA_VERSION};

    const NOW: u64 = 100;
    const DEADLINE: u64 = 1_000;
    const CAPABILITY_GENERATION: &str = "test-capability-generation";

    fn id(value: &str) -> OpaqueId {
        OpaqueId::new(value).unwrap()
    }

    fn scoped(source: SourceKind, value: &str) -> SourceScopedId {
        SourceScopedId::new(source, id(value))
    }

    fn request(source: SourceKind, request_id: &str, session_id: &str) -> PendingRequest {
        PendingRequest {
            schema_version: SchemaVersion::current(),
            identity: scoped(source, request_id),
            session_id: Some(scoped(source, session_id)),
            thread_id: Some(id("thread-1")),
            turn_id: Some(id("turn-1")),
            tool_item_id: Some(id("tool-1")),
            agent_id: None,
            kind: RequestKind::Approval,
            requested_capability: Some(id("filesystem.write")),
            lifecycle: RequestLifecycle::Pending,
            created_at_unix_ms: Some(NOW),
            deadline_unix_ms: Some(DEADLINE),
        }
    }

    fn registry(enabled: bool, generation: &str) -> CapabilityRegistry {
        CapabilityRegistry {
            schema_version: REGISTRY_SCHEMA_VERSION,
            generation: generation.into(),
            runtime_source: "windows-package-discovery".into(),
            fixture_mode: false,
            installed_codex_desktop_version: None,
            capabilities: vec![CapabilityEntry {
                id: PERMISSION_DECISION_CAPABILITY.into(),
                surface: "test-only simulated permission decision".into(),
                supported_version: None,
                transport: "test only".into(),
                evidence: "unit test only; not runtime evidence".into(),
                test_reference: "request_router tests".into(),
                failure_behavior: "deny".into(),
                fallback: "resolve in Codex".into(),
                adapter_available: enabled,
                enabled,
                disabled_reason: (!enabled).then(|| "disabled in test".into()),
            }],
        }
    }

    fn presented(router: &mut RequestRouter, request: &PendingRequest) -> PresentationToken {
        router.register(request, NOW).unwrap();
        let token = router.begin_presentation(request, NOW).unwrap();
        router
            .acknowledge_presentation(request, &token, NOW)
            .unwrap();
        token
    }

    #[test]
    fn request_identity_is_source_scoped_and_conflicting_scope_is_rejected() {
        let mut router = RequestRouter::new();
        let codex = request(SourceKind::CodexCliObserver, "request-1", "session-a");
        let git = request(SourceKind::LocalGit, "request-1", "session-a");
        assert_eq!(router.register(&codex, NOW), Ok(Registration::Inserted));
        assert_eq!(router.register(&git, NOW), Ok(Registration::Inserted));

        let same_id_different_session =
            request(SourceKind::CodexCliObserver, "request-1", "session-b");
        assert_eq!(
            router.register(&same_id_different_session, NOW),
            Err(RouteError::ConflictingRequestIdentity)
        );
        assert_eq!(router.tracked_requests(), 2);
    }

    #[test]
    fn registration_is_idempotent_without_resetting_route_state() {
        let mut router = RequestRouter::new();
        let request = request(SourceKind::CodexCliObserver, "request-1", "session-1");
        assert_eq!(router.register(&request, NOW), Ok(Registration::Inserted));
        let token = router.begin_presentation(&request, NOW).unwrap();
        router
            .acknowledge_presentation(&request, &token, NOW)
            .unwrap();
        assert_eq!(
            router.register(&request, NOW),
            Ok(Registration::AlreadyRegistered)
        );
        assert_eq!(
            router.acknowledge_presentation(&request, &token, NOW),
            Err(RouteError::AlreadyAcknowledged)
        );
    }

    #[test]
    fn missing_or_elapsed_source_deadline_fails_closed() {
        let mut router = RequestRouter::new();
        let mut no_deadline = request(SourceKind::CodexCliObserver, "request-1", "session-1");
        no_deadline.deadline_unix_ms = None;
        assert_eq!(
            router.register(&no_deadline, NOW),
            Err(RouteError::MissingDeadline)
        );

        let request = request(SourceKind::CodexCliObserver, "request-2", "session-1");
        assert_eq!(
            router.register(&request, DEADLINE),
            Err(RouteError::DeadlineElapsed)
        );
        assert_eq!(router.tracked_requests(), 0);
    }

    #[test]
    fn session_source_must_match_exact_request_source() {
        let mut router = RequestRouter::new();
        let mut request = request(SourceKind::CodexCliObserver, "request-1", "session-1");
        request.session_id = Some(scoped(SourceKind::LocalGit, "session-1"));
        assert_eq!(
            router.register(&request, NOW),
            Err(RouteError::InvalidSourceScope)
        );
    }

    #[test]
    fn changes_to_correlated_scope_ids_conflict_with_the_registered_request() {
        let mut router = RequestRouter::new();
        let request = request(SourceKind::CodexCliObserver, "request-1", "session-1");
        router.register(&request, NOW).unwrap();

        for changed in [
            PendingRequest {
                thread_id: Some(id("thread-2")),
                ..request.clone()
            },
            PendingRequest {
                turn_id: Some(id("turn-2")),
                ..request.clone()
            },
            PendingRequest {
                tool_item_id: Some(id("tool-2")),
                ..request.clone()
            },
            PendingRequest {
                agent_id: Some(id("agent-2")),
                ..request.clone()
            },
        ] {
            assert_eq!(
                router.begin_presentation(&changed, NOW),
                Err(RouteError::ConflictingRequestIdentity)
            );
        }
    }

    #[test]
    fn reply_requires_acknowledged_current_presentation_and_enabled_capability() {
        let mut router = RequestRouter::new();
        let request = request(SourceKind::CodexCliObserver, "request-1", "session-1");
        router.register(&request, NOW).unwrap();
        let stale_registry = registry(true, CAPABILITY_GENERATION);
        let token = router.begin_presentation(&request, NOW).unwrap();
        let stale_token = router.begin_presentation(&request, NOW).unwrap();
        assert_eq!(
            router.acknowledge_presentation(&request, &token, NOW),
            Err(RouteError::StalePresentation)
        );

        let disabled = registry(false, CAPABILITY_GENERATION);
        router
            .acknowledge_presentation(&request, &stale_token, NOW)
            .unwrap();
        assert_eq!(
            router.claim_reply(
                &disabled,
                CAPABILITY_GENERATION,
                &request,
                &stale_token,
                PermissionDecision::Allow,
                NOW,
            ),
            Err(RouteError::CapabilityUnavailable)
        );
        assert_eq!(router.in_flight_replies(), 0);

        let stale_generation = registry(true, "old-generation");
        assert_eq!(
            router.claim_reply(
                &stale_generation,
                CAPABILITY_GENERATION,
                &request,
                &stale_token,
                PermissionDecision::Allow,
                NOW,
            ),
            Err(RouteError::CapabilityUnavailable)
        );
        assert_eq!(router.in_flight_replies(), 0);

        let attempt = router
            .claim_reply(
                &stale_registry,
                CAPABILITY_GENERATION,
                &request,
                &stale_token,
                PermissionDecision::Allow,
                NOW,
            )
            .unwrap();
        assert_eq!(attempt.decision(), PermissionDecision::Allow);
        assert_eq!(router.in_flight_replies(), 1);
    }

    #[test]
    fn request_deadline_is_checked_again_before_reply_reservation() {
        let mut router = RequestRouter::new();
        let request = request(SourceKind::CodexCliObserver, "request-1", "session-1");
        let token = presented(&mut router, &request);
        assert_eq!(
            router.claim_reply(
                &registry(true, CAPABILITY_GENERATION),
                CAPABILITY_GENERATION,
                &request,
                &token,
                PermissionDecision::Deny,
                DEADLINE,
            ),
            Err(RouteError::DeadlineElapsed)
        );
        assert_eq!(router.in_flight_replies(), 0);
    }

    #[test]
    fn unsupported_request_kinds_cannot_use_permission_decisions() {
        let mut router = RequestRouter::new();
        let mut request = request(SourceKind::CodexCliObserver, "request-1", "session-1");
        request.kind = RequestKind::UserInput;
        let token = presented(&mut router, &request);
        assert_eq!(
            router.claim_reply(
                &registry(true, CAPABILITY_GENERATION),
                CAPABILITY_GENERATION,
                &request,
                &token,
                PermissionDecision::Deny,
                NOW,
            ),
            Err(RouteError::UnsupportedRequestKind)
        );
        assert_eq!(router.in_flight_replies(), 0);
    }

    #[test]
    fn concurrent_attempts_are_bounded_and_completed_slots_can_be_reused() {
        let mut router = RequestRouter::new();
        let registry = registry(true, CAPABILITY_GENERATION);
        let mut attempts = Vec::new();
        let mut requests = Vec::new();
        let mut tokens = Vec::new();
        for index in 0..=MAX_CONCURRENT_REPLY_ATTEMPTS {
            let request = request(
                SourceKind::CodexCliObserver,
                &format!("request-{index}"),
                &format!("session-{index}"),
            );
            let token = presented(&mut router, &request);
            requests.push(request);
            tokens.push(token);
        }
        for index in 0..MAX_CONCURRENT_REPLY_ATTEMPTS {
            attempts.push(
                router
                    .claim_reply(
                        &registry,
                        CAPABILITY_GENERATION,
                        &requests[index],
                        &tokens[index],
                        PermissionDecision::Allow,
                        NOW,
                    )
                    .unwrap(),
            );
        }
        assert_eq!(router.in_flight_replies(), MAX_CONCURRENT_REPLY_ATTEMPTS);
        assert_eq!(
            router.claim_reply(
                &registry,
                CAPABILITY_GENERATION,
                &requests[MAX_CONCURRENT_REPLY_ATTEMPTS],
                &tokens[MAX_CONCURRENT_REPLY_ATTEMPTS],
                PermissionDecision::Allow,
                NOW,
            ),
            Err(RouteError::ReplyConcurrencyLimit)
        );

        router
            .complete_reply(attempts.remove(0), DeliveryOutcome::AcceptedByAdapter)
            .unwrap();
        assert!(
            router
                .claim_reply(
                    &registry,
                    CAPABILITY_GENERATION,
                    &requests[MAX_CONCURRENT_REPLY_ATTEMPTS],
                    &tokens[MAX_CONCURRENT_REPLY_ATTEMPTS],
                    PermissionDecision::Allow,
                    NOW,
                )
                .is_ok()
        );
    }

    #[test]
    fn duplicate_or_uncertain_attempts_are_never_replayed() {
        let mut router = RequestRouter::new();
        let request = request(SourceKind::CodexCliObserver, "request-1", "session-1");
        let token = presented(&mut router, &request);
        let registry = registry(true, CAPABILITY_GENERATION);
        let attempt = router
            .claim_reply(
                &registry,
                CAPABILITY_GENERATION,
                &request,
                &token,
                PermissionDecision::Deny,
                NOW,
            )
            .unwrap();
        let completion = router
            .complete_reply(attempt, DeliveryOutcome::Uncertain)
            .unwrap();
        assert_eq!(completion.outcome, DeliveryOutcome::Uncertain);
        assert_eq!(router.in_flight_replies(), 0);
        assert_eq!(
            router.claim_reply(
                &registry,
                CAPABILITY_GENERATION,
                &request,
                &token,
                PermissionDecision::Deny,
                NOW,
            ),
            Err(RouteError::ReplyAlreadyClaimed)
        );
    }

    #[test]
    fn every_completion_outcome_is_terminal_for_its_request() {
        let registry = registry(true, CAPABILITY_GENERATION);
        for (index, outcome) in [
            DeliveryOutcome::AcceptedByAdapter,
            DeliveryOutcome::RejectedBeforeSend,
            DeliveryOutcome::Uncertain,
        ]
        .into_iter()
        .enumerate()
        {
            let mut router = RequestRouter::new();
            let request = request(
                SourceKind::CodexCliObserver,
                &format!("request-{index}"),
                &format!("session-{index}"),
            );
            let presentation = presented(&mut router, &request);
            let attempt = router
                .claim_reply(
                    &registry,
                    CAPABILITY_GENERATION,
                    &request,
                    &presentation,
                    PermissionDecision::Deny,
                    NOW,
                )
                .unwrap();
            assert_eq!(
                router.complete_reply(attempt, outcome).unwrap().outcome,
                outcome
            );
            assert_eq!(
                router.claim_reply(
                    &registry,
                    CAPABILITY_GENERATION,
                    &request,
                    &presentation,
                    PermissionDecision::Deny,
                    NOW,
                ),
                Err(RouteError::ReplyAlreadyClaimed)
            );
        }
    }

    #[test]
    fn terminal_source_updates_clear_routes_and_reject_late_completions() {
        let registry = registry(true, CAPABILITY_GENERATION);
        for (index, lifecycle) in [
            RequestLifecycle::Resolved,
            RequestLifecycle::Expired,
            RequestLifecycle::Cancelled,
        ]
        .into_iter()
        .enumerate()
        {
            let mut router = RequestRouter::new();
            let request = request(
                SourceKind::CodexCliObserver,
                &format!("request-{index}"),
                &format!("session-{index}"),
            );
            let presentation = presented(&mut router, &request);
            let attempt = router
                .claim_reply(
                    &registry,
                    CAPABILITY_GENERATION,
                    &request,
                    &presentation,
                    PermissionDecision::Deny,
                    NOW,
                )
                .unwrap();
            let mut terminal = request;
            terminal.lifecycle = lifecycle;

            assert_eq!(
                router.clear_terminal_update(&terminal),
                Ok(TerminalCleanup::Removed)
            );
            assert_eq!(router.tracked_requests(), 0);
            assert_eq!(router.in_flight_replies(), 0);
            assert_eq!(
                router.complete_reply(attempt, DeliveryOutcome::AcceptedByAdapter),
                Err(RouteError::UnknownRequest)
            );
            assert_eq!(
                router.clear_terminal_update(&terminal),
                Ok(TerminalCleanup::NotTracked)
            );
        }
    }

    #[test]
    fn failed_delivery_stays_pending_until_external_resolution_reaches_snapshot() {
        let mut router = RequestRouter::new();
        let broker = BrokerService::default();
        let pending = request(SourceKind::CodexCliObserver, "request-1", "session-1");
        let (created, _) = broker
            .apply(
                BrokerUpdate::UpsertPendingRequest(pending.clone()),
                NOW,
                None,
                None,
            )
            .unwrap();
        assert_eq!(created.sequence, 1);

        let presentation = presented(&mut router, &pending);
        let attempt = router
            .claim_reply(
                &registry(true, CAPABILITY_GENERATION),
                CAPABILITY_GENERATION,
                &pending,
                &presentation,
                PermissionDecision::Allow,
                NOW,
            )
            .unwrap();
        assert_eq!(
            router.complete_reply(attempt, DeliveryOutcome::RejectedBeforeSend),
            Ok(ReplyCompletion {
                request: presentation.key.clone(),
                outcome: DeliveryOutcome::RejectedBeforeSend,
            })
        );
        assert_eq!(router.tracked_requests(), 1);

        let BrokerSyncResponse::Snapshot { snapshot } = broker.synchronize_after(0).unwrap() else {
            panic!("fresh broker consumers must receive a snapshot");
        };
        assert_eq!(snapshot.sequence, 1);
        assert_eq!(snapshot.pending_requests, vec![pending.clone()]);

        // A different client may resolve the request. Only that authoritative
        // terminal record clears both router tracking and broker snapshot state.
        let mut resolved = pending;
        resolved.lifecycle = RequestLifecycle::Resolved;
        assert_eq!(
            router.clear_terminal_update(&resolved),
            Ok(TerminalCleanup::Removed)
        );
        let (terminal, _) = broker
            .apply(
                BrokerUpdate::UpsertPendingRequest(resolved),
                NOW + 1,
                None,
                None,
            )
            .unwrap();
        assert_eq!(terminal.sequence, 2);

        let BrokerSyncResponse::Snapshot { snapshot } = broker.synchronize_after(0).unwrap() else {
            panic!("fresh broker consumers must receive a snapshot");
        };
        assert_eq!(snapshot.sequence, 2);
        assert!(snapshot.pending_requests.is_empty());
        assert_eq!(router.tracked_requests(), 0);
    }

    #[test]
    fn conflicting_or_nonterminal_updates_do_not_clear_a_route() {
        let mut router = RequestRouter::new();
        let request = request(SourceKind::CodexCliObserver, "request-1", "session-1");
        router.register(&request, NOW).unwrap();

        let mut changed_scope = request.clone();
        changed_scope.turn_id = Some(id("different-turn"));
        changed_scope.lifecycle = RequestLifecycle::Resolved;
        assert_eq!(
            router.clear_terminal_update(&changed_scope),
            Err(RouteError::ConflictingRequestIdentity)
        );

        assert_eq!(
            router.clear_terminal_update(&request),
            Err(RouteError::TerminalUpdateRequired)
        );
        assert_eq!(router.tracked_requests(), 1);
    }

    #[test]
    fn tracking_capacity_rejects_without_eviction() {
        let mut router = RequestRouter::new();
        for index in 0..MAX_TRACKED_REQUESTS {
            let request = request(
                SourceKind::CodexCliObserver,
                &format!("request-{index}"),
                &format!("session-{index}"),
            );
            assert_eq!(router.register(&request, NOW), Ok(Registration::Inserted));
        }
        let overflow = request(SourceKind::CodexCliObserver, "overflow", "session-overflow");
        assert_eq!(
            router.register(&overflow, NOW),
            Err(RouteError::RequestCapacityReached)
        );
        assert_eq!(router.tracked_requests(), MAX_TRACKED_REQUESTS);

        let mut cancelled = request(SourceKind::CodexCliObserver, "request-0", "session-0");
        cancelled.lifecycle = RequestLifecycle::Cancelled;
        assert_eq!(
            router.clear_terminal_update(&cancelled),
            Ok(TerminalCleanup::Removed)
        );
        assert_eq!(router.register(&overflow, NOW), Ok(Registration::Inserted));
        assert_eq!(router.tracked_requests(), MAX_TRACKED_REQUESTS);
    }
}
