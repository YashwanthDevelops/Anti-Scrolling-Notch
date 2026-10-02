//! Versioned domain records for the backend state pipeline.
//!
//! The current Codex CLI observer still accepts only the four captured event
//! names through `codex_hook_contract::CodexHookObservation`. These records do
//! not infer a session, turn, tool, request, or identifier from that anonymous
//! observation. Representable states are not evidence that a source adapter is
//! verified or enabled.

use std::fmt;
use std::num::NonZeroU64;

use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize};

pub const DOMAIN_SCHEMA_VERSION: u16 = 1;
pub const MAX_OPAQUE_ID_BYTES: usize = 512;

/// Domain-record schema version. This is independent of the Codex pipe's
/// `wire_version`; version 1 is the only supported domain schema today.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct SchemaVersion(u16);

impl SchemaVersion {
    pub const fn current() -> Self {
        Self(DOMAIN_SCHEMA_VERSION)
    }

    pub const fn get(self) -> u16 {
        self.0
    }
}

impl TryFrom<u16> for SchemaVersion {
    type Error = ValidationError;

    fn try_from(version: u16) -> Result<Self, Self::Error> {
        if version == DOMAIN_SCHEMA_VERSION {
            Ok(Self(version))
        } else {
            Err(ValidationError::UnsupportedSchemaVersion(version))
        }
    }
}

impl<'de> Deserialize<'de> for SchemaVersion {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let version = u16::deserialize(deserializer)?;
        Self::try_from(version).map_err(D::Error::custom)
    }
}

/// An exact, bounded identifier supplied by a source or assigned by the app.
/// Values are preserved verbatim; callers must not concatenate IDs to form a
/// composite key. Pair an ID with `SourceKind` using `SourceScopedId` instead.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct OpaqueId(String);

impl OpaqueId {
    pub fn new(value: impl Into<String>) -> Result<Self, ValidationError> {
        let value = value.into();
        if value.is_empty() {
            return Err(ValidationError::EmptyId);
        }
        if value.len() > MAX_OPAQUE_ID_BYTES {
            return Err(ValidationError::IdTooLong);
        }
        if value.chars().any(char::is_control) {
            return Err(ValidationError::IdContainsControlCharacter);
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl<'de> Deserialize<'de> for OpaqueId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Self::new(String::deserialize(deserializer)?).map_err(D::Error::custom)
    }
}

/// Validation failures shared by bounded IDs and event sequence values.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ValidationError {
    UnsupportedSchemaVersion(u16),
    EmptyId,
    IdTooLong,
    IdContainsControlCharacter,
    ZeroEventSequence,
}

impl fmt::Display for ValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedSchemaVersion(version) => {
                write!(formatter, "unsupported domain schema version {version}")
            }
            Self::EmptyId => formatter.write_str("identifier must not be empty"),
            Self::IdTooLong => write!(
                formatter,
                "identifier exceeds the {MAX_OPAQUE_ID_BYTES}-byte limit"
            ),
            Self::IdContainsControlCharacter => {
                formatter.write_str("identifier contains a control character")
            }
            Self::ZeroEventSequence => formatter.write_str("event sequence must be non-zero"),
        }
    }
}

impl std::error::Error for ValidationError {}

/// Broad internal producer category. These labels do not mean that a monitor
/// for the corresponding source has been implemented or enabled.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceKind {
    CodexCliObserver,
    LocalGit,
    #[serde(rename = "github")]
    GitHub,
    Windows,
    Integration,
}

/// Source identity is a pair, not a concatenated string, so equal upstream IDs
/// from separate sources remain distinct.
#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct SourceScopedId {
    pub source: SourceKind,
    pub id: OpaqueId,
}

impl SourceScopedId {
    pub fn new(source: SourceKind, id: OpaqueId) -> Self {
        Self { source, id }
    }
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionLifecycle {
    #[default]
    Unknown,
    Active,
    Ended,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionActivity {
    #[default]
    Unknown,
    Idle,
    Working,
}

/// Waiting classification is backend state, not a recognized Codex hook event.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WaitingState {
    #[default]
    Unknown,
    NotWaiting,
    UserInput,
    Approval,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ConnectionHealth {
    #[default]
    Unknown,
    Healthy,
    Degraded,
    Offline,
}

/// One independently identified source session. All IDs remain optional when
/// their source has not supplied or verified them; the record identity itself
/// is therefore always an exact source-scoped key.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Session {
    pub schema_version: SchemaVersion,
    pub identity: SourceScopedId,
    pub thread_id: Option<OpaqueId>,
    pub agent_id: Option<OpaqueId>,
    pub parent_agent_id: Option<OpaqueId>,
    pub repository_id: Option<SourceScopedId>,
    pub worktree_id: Option<OpaqueId>,
    pub display_name: Option<String>,
    pub model: Option<String>,
    pub lifecycle: SessionLifecycle,
    pub activity: SessionActivity,
    pub waiting: WaitingState,
    pub connection: ConnectionHealth,
    pub started_at_unix_ms: Option<u64>,
    pub last_seen_at_unix_ms: Option<u64>,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TurnLifecycle {
    #[default]
    Unknown,
    Active,
    Completed,
    Interrupted,
    Failed,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TurnOutcome {
    #[default]
    Unknown,
    Success,
    Failure,
    Interrupted,
}

/// A turn's outcome is separate from lifecycle. In particular, an observed
/// Codex `Stop` hook is not sufficient evidence of a successful outcome.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Turn {
    pub schema_version: SchemaVersion,
    pub identity: SourceScopedId,
    pub session_id: SourceScopedId,
    pub thread_id: Option<OpaqueId>,
    pub agent_id: Option<OpaqueId>,
    pub lifecycle: TurnLifecycle,
    pub outcome: TurnOutcome,
    pub started_at_unix_ms: Option<u64>,
    pub ended_at_unix_ms: Option<u64>,
    /// Display content; persistence must be governed by CORE-04 retention and
    /// redaction policy. This packet does not persist or populate it.
    pub summary: Option<String>,
    pub plan: Option<String>,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolItemState {
    #[default]
    Unknown,
    Active,
    Completed,
    Failed,
    Cancelled,
}

/// Normalized tool metadata only. It intentionally has no raw arguments or
/// output fields; source-specific tool schemas require compatibility evidence.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ToolItem {
    pub schema_version: SchemaVersion,
    pub identity: SourceScopedId,
    pub session_id: SourceScopedId,
    pub turn_id: Option<SourceScopedId>,
    pub agent_id: Option<OpaqueId>,
    pub display_name: Option<String>,
    pub sanitized_target: Option<String>,
    pub state: ToolItemState,
    pub started_at_unix_ms: Option<u64>,
    pub ended_at_unix_ms: Option<u64>,
    pub exit_code: Option<i32>,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RequestKind {
    #[default]
    Unknown,
    Approval,
    UserInput,
    Elicitation,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RequestLifecycle {
    #[default]
    Unknown,
    Pending,
    Presented,
    Submitting,
    Resolved,
    Expired,
    Cancelled,
}

/// A request record is representational only. Its existence does not authorize
/// Allow/Deny, input, elicitation, or any other unverified Codex response path.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct PendingRequest {
    pub schema_version: SchemaVersion,
    pub identity: SourceScopedId,
    pub session_id: Option<SourceScopedId>,
    pub thread_id: Option<OpaqueId>,
    pub turn_id: Option<OpaqueId>,
    pub tool_item_id: Option<OpaqueId>,
    pub agent_id: Option<OpaqueId>,
    pub kind: RequestKind,
    pub requested_capability: Option<OpaqueId>,
    pub lifecycle: RequestLifecycle,
    pub created_at_unix_ms: Option<u64>,
    pub deadline_unix_ms: Option<u64>,
}

/// Git host identity is separate from the local worktree, branch and HEAD.
/// This model intentionally excludes local paths and credential-bearing URLs.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Repository {
    pub schema_version: SchemaVersion,
    pub identity: SourceScopedId,
    pub host: Option<String>,
    pub owner: Option<String>,
    pub name: Option<String>,
    pub worktree_id: Option<OpaqueId>,
    pub branch: Option<String>,
    pub head_sha: Option<String>,
    pub observed_at_unix_ms: Option<u64>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum IntegrationProvider {
    #[serde(rename = "github")]
    GitHub,
    Vercel,
    #[serde(rename = "n8n")]
    N8n,
    Stripe,
    Resend,
    Notion,
    #[serde(rename = "cal_com")]
    CalCom,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ConfigurationState {
    #[default]
    Unknown,
    Unconfigured,
    Configured,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum IntegrationErrorKind {
    Authentication,
    Permission,
    RateLimited,
    Network,
    InvalidResponse,
    Other,
}

/// Typed integration health metadata. Credentials, arbitrary response JSON and
/// free-form transport errors belong outside this record.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Integration {
    pub schema_version: SchemaVersion,
    pub identity: OpaqueId,
    pub provider: IntegrationProvider,
    pub configuration: ConfigurationState,
    pub connection: ConnectionHealth,
    pub last_success_at_unix_ms: Option<u64>,
    pub data_revision: Option<u64>,
    pub retry_at_unix_ms: Option<u64>,
    pub last_error: Option<IntegrationErrorKind>,
    pub unread_event_ids: Vec<OpaqueId>,
}

/// Non-zero local event sequence. CORE-03 will define allocation/replay; this
/// type only makes invalid zero-valued envelopes unrepresentable.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize)]
#[serde(transparent)]
pub struct EventSequence(NonZeroU64);

impl EventSequence {
    pub fn new(value: u64) -> Result<Self, ValidationError> {
        NonZeroU64::new(value)
            .map(Self)
            .ok_or(ValidationError::ZeroEventSequence)
    }

    pub const fn get(self) -> u64 {
        self.0.get()
    }
}

impl<'de> Deserialize<'de> for EventSequence {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Self::new(u64::deserialize(deserializer)?).map_err(D::Error::custom)
    }
}

/// Optional source identities associated with an event. Missing values stay
/// missing; adapters must not synthesize session or turn IDs from an event name.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct EventCorrelation {
    pub session_id: Option<SourceScopedId>,
    pub thread_id: Option<SourceScopedId>,
    pub turn_id: Option<SourceScopedId>,
    pub tool_item_id: Option<SourceScopedId>,
    pub agent_id: Option<SourceScopedId>,
    pub repository_id: Option<SourceScopedId>,
}

/// Internal normalized event record, separate from the hook pipe's
/// `wire_version`/`event` message. `payload` is generic so this packet does not
/// invent source events; the accepted hook observation is the only event
/// payload exercised here. CORE-03 will own sequence assignment and replay.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct EventEnvelope<T> {
    pub schema_version: SchemaVersion,
    pub sequence: EventSequence,
    pub source: SourceKind,
    pub source_event_id: Option<OpaqueId>,
    pub observed_at_unix_ms: u64,
    pub correlation: EventCorrelation,
    pub deduplication_key: Option<OpaqueId>,
    pub payload: T,
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use codex_hook_contract::{CodexHookEvent, CodexHookObservation};
    use serde::de::DeserializeOwned;
    use serde::Serialize;

    use super::*;

    fn id(value: &str) -> OpaqueId {
        OpaqueId::new(value).unwrap()
    }

    fn scoped(source: SourceKind, value: &str) -> SourceScopedId {
        SourceScopedId::new(source, id(value))
    }

    fn round_trip<T>(value: &T)
    where
        T: DeserializeOwned + PartialEq + fmt::Debug + Serialize,
    {
        let serialized = serde_json::to_vec(value).unwrap();
        let decoded = serde_json::from_slice::<T>(&serialized).unwrap();
        assert_eq!(&decoded, value);
    }

    #[test]
    fn all_seven_domain_records_round_trip_as_schema_version_one() {
        let session = Session {
            schema_version: SchemaVersion::current(),
            identity: scoped(SourceKind::CodexCliObserver, "session-exact-1"),
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
        };
        let turn = Turn {
            schema_version: SchemaVersion::current(),
            identity: scoped(SourceKind::CodexCliObserver, "turn-exact-1"),
            session_id: scoped(SourceKind::CodexCliObserver, "session-exact-1"),
            thread_id: None,
            agent_id: None,
            lifecycle: TurnLifecycle::Unknown,
            outcome: TurnOutcome::Unknown,
            started_at_unix_ms: None,
            ended_at_unix_ms: None,
            summary: None,
            plan: None,
        };
        let tool_item = ToolItem {
            schema_version: SchemaVersion::current(),
            identity: scoped(SourceKind::CodexCliObserver, "tool-exact-1"),
            session_id: scoped(SourceKind::CodexCliObserver, "session-exact-1"),
            turn_id: None,
            agent_id: None,
            display_name: None,
            sanitized_target: None,
            state: ToolItemState::Unknown,
            started_at_unix_ms: None,
            ended_at_unix_ms: None,
            exit_code: None,
        };
        let pending_request = PendingRequest {
            schema_version: SchemaVersion::current(),
            identity: scoped(SourceKind::CodexCliObserver, "request-exact-1"),
            session_id: None,
            thread_id: None,
            turn_id: None,
            tool_item_id: None,
            agent_id: None,
            kind: RequestKind::Unknown,
            requested_capability: None,
            lifecycle: RequestLifecycle::Unknown,
            created_at_unix_ms: None,
            deadline_unix_ms: None,
        };
        let repository = Repository {
            schema_version: SchemaVersion::current(),
            identity: scoped(SourceKind::LocalGit, "repository-local-1"),
            host: None,
            owner: None,
            name: None,
            worktree_id: None,
            branch: None,
            head_sha: None,
            observed_at_unix_ms: None,
        };
        let integration = Integration {
            schema_version: SchemaVersion::current(),
            identity: id("integration_github"),
            provider: IntegrationProvider::GitHub,
            configuration: ConfigurationState::Unknown,
            connection: ConnectionHealth::Unknown,
            last_success_at_unix_ms: None,
            data_revision: None,
            retry_at_unix_ms: None,
            last_error: None,
            unread_event_ids: Vec::new(),
        };
        let envelope = EventEnvelope {
            schema_version: SchemaVersion::current(),
            sequence: EventSequence::new(1).unwrap(),
            source: SourceKind::CodexCliObserver,
            source_event_id: None,
            observed_at_unix_ms: 1_791_000_000_000,
            correlation: EventCorrelation::default(),
            deduplication_key: None,
            payload: CodexHookObservation::new(CodexHookEvent::Stop),
        };

        round_trip(&session);
        round_trip(&turn);
        round_trip(&tool_item);
        round_trip(&pending_request);
        round_trip(&repository);
        round_trip(&integration);
        round_trip(&envelope);

        for record in [
            serde_json::to_value(session).unwrap(),
            serde_json::to_value(turn).unwrap(),
            serde_json::to_value(tool_item).unwrap(),
            serde_json::to_value(pending_request).unwrap(),
            serde_json::to_value(repository).unwrap(),
            serde_json::to_value(integration).unwrap(),
        ] {
            assert_eq!(record["schemaVersion"], DOMAIN_SCHEMA_VERSION);
        }
        let envelope_json = serde_json::to_value(envelope).unwrap();
        assert_eq!(envelope_json["schemaVersion"], DOMAIN_SCHEMA_VERSION);
        assert_eq!(envelope_json["payload"]["wire_version"], 1);
        assert_eq!(envelope_json["payload"]["event"], "Stop");
    }

    #[test]
    fn unsupported_or_missing_domain_versions_and_unknown_fields_are_rejected() {
        let base = serde_json::json!({
            "schemaVersion": DOMAIN_SCHEMA_VERSION,
            "identity": {"source": "codex_cli_observer", "id": "session-1"},
            "threadId": null,
            "agentId": null,
            "parentAgentId": null,
            "repositoryId": null,
            "worktreeId": null,
            "displayName": null,
            "model": null,
            "lifecycle": "unknown",
            "activity": "unknown",
            "waiting": "unknown",
            "connection": "unknown",
            "startedAtUnixMs": null,
            "lastSeenAtUnixMs": null
        });
        assert!(serde_json::from_value::<Session>(base.clone()).is_ok());

        let mut unsupported = base.clone();
        unsupported["schemaVersion"] = serde_json::json!(DOMAIN_SCHEMA_VERSION + 1);
        assert!(serde_json::from_value::<Session>(unsupported).is_err());

        let mut missing = base.clone();
        missing.as_object_mut().unwrap().remove("schemaVersion");
        assert!(serde_json::from_value::<Session>(missing).is_err());

        let mut unknown = base;
        unknown["futureField"] = serde_json::json!(true);
        assert!(serde_json::from_value::<Session>(unknown).is_err());
    }

    #[test]
    fn unknown_internal_enum_values_fail_closed() {
        assert!(serde_json::from_str::<TurnLifecycle>("\"succeeded\"").is_err());
        assert!(serde_json::from_str::<SourceKind>("\"codex_desktop_monitor\"").is_err());
        assert!(serde_json::from_str::<IntegrationProvider>("\"unknown_service\"").is_err());
    }

    #[test]
    fn source_scoped_ids_keep_equal_values_from_different_sources_distinct() {
        let cli = scoped(SourceKind::CodexCliObserver, "same-source-id");
        let git = scoped(SourceKind::GitHub, "same-source-id");
        assert_ne!(cli, git);
        assert_eq!(cli.id.as_str(), git.id.as_str());
        assert_eq!(HashSet::from([cli, git]).len(), 2);
    }

    #[test]
    fn opaque_ids_are_bounded_and_are_not_normalized_or_silently_repaired() {
        assert!(matches!(OpaqueId::new(""), Err(ValidationError::EmptyId)));
        assert!(matches!(
            OpaqueId::new("invalid\nidentity"),
            Err(ValidationError::IdContainsControlCharacter)
        ));
        assert!(matches!(
            OpaqueId::new("x".repeat(MAX_OPAQUE_ID_BYTES + 1)),
            Err(ValidationError::IdTooLong)
        ));
        assert_eq!(OpaqueId::new(" id ").unwrap().as_str(), " id ");
        assert!(serde_json::from_str::<OpaqueId>("\"\"").is_err());
    }

    #[test]
    fn event_sequence_rejects_zero_and_wraps_the_existing_observation_without_mutating_it() {
        assert_eq!(
            EventSequence::new(0),
            Err(ValidationError::ZeroEventSequence)
        );
        assert!(serde_json::from_str::<EventSequence>("0").is_err());

        let envelope = EventEnvelope {
            schema_version: SchemaVersion::current(),
            sequence: EventSequence::new(12).unwrap(),
            source: SourceKind::CodexCliObserver,
            source_event_id: None,
            observed_at_unix_ms: 1_791_000_000_000,
            correlation: EventCorrelation::default(),
            deduplication_key: None,
            payload: CodexHookObservation::new(CodexHookEvent::UserPromptSubmit),
        };
        let value = serde_json::to_value(envelope).unwrap();
        assert_eq!(
            value["payload"],
            serde_json::json!({"wire_version": 1, "event": "UserPromptSubmit"})
        );
        assert!(value["correlation"]["sessionId"].is_null());
        assert!(value["correlation"]["turnId"].is_null());
    }
}
