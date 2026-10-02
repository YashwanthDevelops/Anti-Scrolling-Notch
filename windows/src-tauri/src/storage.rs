//! Bounded, local-only history for verified observer records.
//!
//! Codex CLI observations use only the four event names accepted by the
//! existing wire contract. Normalized broker updates can be recorded by later
//! packets through the same store; their free-text fields are omitted unless
//! the user explicitly enables content retention.

use std::path::Path;
use std::time::Duration;

use codex_hook_contract::CodexHookObservation;
use rusqlite::{params, Connection, OpenFlags, Transaction};

use crate::broker::reducer::BrokerUpdate;
use crate::broker::types::EventEnvelope;

pub const HISTORY_SCHEMA_VERSION: u32 = 1;
pub const MAX_HISTORY_EVENTS: usize = 10_000;
pub const MAX_HISTORY_RECORD_BYTES: usize = 64 * 1024;
pub const MAX_HISTORY_PAYLOAD_BYTES: usize = 16 * 1024 * 1024;
pub const MAX_HISTORY_DATABASE_BYTES: u64 = 32 * 1024 * 1024;
pub const MAX_HISTORY_READ_PAGE: usize = 500;

const HOOK_OBSERVATION_KIND: &str = "hook_observation";
const BROKER_UPDATE_KIND: &str = "broker_update";

#[derive(Debug)]
pub enum HistoryError {
    Sql(rusqlite::Error),
    Serialization(serde_json::Error),
    Io(std::io::Error),
    UnsupportedSchema(u32),
    DatabaseExceedsBound,
    RecordTooLarge { bytes: usize },
    TimestampOutOfRange,
    InvalidCursor,
    InvalidHookWireVersion(u8),
    InvalidStoredSequence,
}

impl std::fmt::Display for HistoryError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Sql(error) => write!(formatter, "history database error: {error}"),
            Self::Serialization(error) => write!(formatter, "history record encoding failed: {error}"),
            Self::Io(error) => write!(formatter, "history database directory could not be prepared: {error}"),
            Self::UnsupportedSchema(version) => {
                write!(formatter, "unsupported history schema version {version}")
            }
            Self::DatabaseExceedsBound => formatter.write_str(
                "history database is larger than the configured storage bound; it was left unchanged",
            ),
            Self::RecordTooLarge { bytes } => write!(
                formatter,
                "history record is {bytes} bytes; the per-record limit is {MAX_HISTORY_RECORD_BYTES} bytes"
            ),
            Self::TimestampOutOfRange => {
                formatter.write_str("history timestamp is outside the supported range")
            }
            Self::InvalidCursor => formatter.write_str("history cursor must not be negative"),
            Self::InvalidHookWireVersion(version) => {
                write!(formatter, "unsupported Codex hook wire version {version}")
            }
            Self::InvalidStoredSequence => {
                formatter.write_str("history database contains an invalid source sequence")
            }
        }
    }
}

impl std::error::Error for HistoryError {}

impl From<rusqlite::Error> for HistoryError {
    fn from(error: rusqlite::Error) -> Self {
        Self::Sql(error)
    }
}

impl From<serde_json::Error> for HistoryError {
    fn from(error: serde_json::Error) -> Self {
        Self::Serialization(error)
    }
}

impl From<std::io::Error> for HistoryError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

/// A local database row. `id` is a storage cursor, not a source event ID.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HistoryRecord {
    pub id: i64,
    pub observed_at_unix_ms: u64,
    pub source: String,
    pub record_kind: String,
    pub source_sequence: Option<u64>,
    pub record_json: String,
    pub content_retained: bool,
}

pub struct HistoryStore {
    connection: Connection,
}

impl HistoryStore {
    pub fn open(path: &Path) -> Result<Self, HistoryError> {
        let parent = path.parent().ok_or(HistoryError::DatabaseExceedsBound)?;
        std::fs::create_dir_all(parent)?;
        let mut connection = Connection::open_with_flags(
            path,
            OpenFlags::SQLITE_OPEN_READ_WRITE
                | OpenFlags::SQLITE_OPEN_CREATE
                | OpenFlags::SQLITE_OPEN_NOFOLLOW,
        )?;
        connection.busy_timeout(Duration::from_secs(2))?;
        let version: i64 = connection.pragma_query_value(None, "user_version", |row| row.get(0))?;
        if version < 0 || version > i64::from(HISTORY_SCHEMA_VERSION) {
            return Err(HistoryError::UnsupportedSchema(
                u32::try_from(version).unwrap_or(u32::MAX),
            ));
        }
        connection.pragma_update(None, "journal_mode", "DELETE")?;
        connection.pragma_update(None, "synchronous", "FULL")?;
        connection.pragma_update(None, "foreign_keys", "ON")?;
        connection.pragma_update(None, "trusted_schema", "OFF")?;

        let page_size: i64 = connection.pragma_query_value(None, "page_size", |row| row.get(0))?;
        if page_size <= 0 {
            return Err(HistoryError::DatabaseExceedsBound);
        }
        let max_pages = i64::try_from(MAX_HISTORY_DATABASE_BYTES / page_size as u64)
            .map_err(|_| HistoryError::DatabaseExceedsBound)?;
        let effective_max_pages: i64 =
            connection.query_row(&format!("PRAGMA max_page_count = {max_pages}"), [], |row| {
                row.get(0)
            })?;
        let page_count: i64 =
            connection.pragma_query_value(None, "page_count", |row| row.get(0))?;
        if effective_max_pages > max_pages || page_count > max_pages {
            return Err(HistoryError::DatabaseExceedsBound);
        }

        migrate(&mut connection)?;
        Ok(Self { connection })
    }

    /// Persist the exact minimal event delivered across the already verified
    /// Codex observer wire. The record contains only `wire_version` and event.
    pub fn record_hook_observation(
        &mut self,
        observation: CodexHookObservation,
        observed_at_unix_ms: u64,
    ) -> Result<i64, HistoryError> {
        if observation.wire_version != codex_hook_contract::WIRE_VERSION {
            return Err(HistoryError::InvalidHookWireVersion(
                observation.wire_version,
            ));
        }
        self.insert_record(
            observed_at_unix_ms,
            "codex_cli_observer",
            HOOK_OBSERVATION_KIND,
            None,
            &serde_json::to_string(&observation)?,
            false,
        )
    }

    /// Persist a normalized broker update without inferring any upstream
    /// fields. Free-text display fields are stripped by default; the explicit
    /// preference retains those fields only after credential/path redaction.
    pub fn record_broker_event(
        &mut self,
        event: &EventEnvelope<BrokerUpdate>,
        retain_content: bool,
    ) -> Result<i64, HistoryError> {
        let mut event = event.clone();
        sanitize_broker_update(&mut event.payload, retain_content);
        let source = serde_json::to_value(event.source)?
            .as_str()
            .unwrap_or("unknown")
            .to_owned();
        let json = serde_json::to_string(&event)?;
        self.insert_record(
            event.observed_at_unix_ms,
            &source,
            BROKER_UPDATE_KIND,
            Some(event.sequence.get()),
            &json,
            retain_content,
        )
    }

    /// Read records after a local row cursor, oldest first. The page size is
    /// hard-capped even if a caller requests an unbounded read.
    pub fn records_after(
        &self,
        after_id: i64,
        requested_limit: usize,
    ) -> Result<Vec<HistoryRecord>, HistoryError> {
        if after_id < 0 {
            return Err(HistoryError::InvalidCursor);
        }
        if requested_limit == 0 {
            return Ok(Vec::new());
        }
        let limit = requested_limit.clamp(1, MAX_HISTORY_READ_PAGE) as i64;
        let mut statement = self.connection.prepare(
            "SELECT id, observed_at_unix_ms, source, record_kind, source_sequence, record_json, content_retained
             FROM history_events WHERE id > ?1 ORDER BY id ASC LIMIT ?2",
        )?;
        let rows = statement.query_map(params![after_id, limit], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, Option<Vec<u8>>>(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, i64>(6)? != 0,
            ))
        })?;

        rows.map(|row| {
            let (id, observed_at, source, record_kind, sequence, record_json, content_retained) =
                row?;
            let observed_at_unix_ms =
                u64::try_from(observed_at).map_err(|_| HistoryError::TimestampOutOfRange)?;
            let source_sequence = sequence
                .map(|bytes| {
                    let bytes: [u8; 8] = bytes
                        .try_into()
                        .map_err(|_| HistoryError::InvalidStoredSequence)?;
                    Ok::<u64, HistoryError>(u64::from_be_bytes(bytes))
                })
                .transpose()?;
            Ok(HistoryRecord {
                id,
                observed_at_unix_ms,
                source,
                record_kind,
                source_sequence,
                record_json,
                content_retained,
            })
        })
        .collect()
    }

    pub fn len(&self) -> Result<usize, HistoryError> {
        let count: i64 =
            self.connection
                .query_row("SELECT COUNT(*) FROM history_events", [], |row| row.get(0))?;
        usize::try_from(count).map_err(|_| HistoryError::DatabaseExceedsBound)
    }

    pub fn is_empty(&self) -> Result<bool, HistoryError> {
        Ok(self.len()? == 0)
    }

    fn insert_record(
        &mut self,
        observed_at_unix_ms: u64,
        source: &str,
        record_kind: &str,
        source_sequence: Option<u64>,
        record_json: &str,
        content_retained: bool,
    ) -> Result<i64, HistoryError> {
        let observed_at =
            i64::try_from(observed_at_unix_ms).map_err(|_| HistoryError::TimestampOutOfRange)?;
        if record_json.len() > MAX_HISTORY_RECORD_BYTES {
            return Err(HistoryError::RecordTooLarge {
                bytes: record_json.len(),
            });
        }
        let sequence_bytes = source_sequence.map(u64::to_be_bytes).map(Vec::from);
        let transaction = self
            .connection
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        prune_history(
            &transaction,
            MAX_HISTORY_EVENTS,
            MAX_HISTORY_PAYLOAD_BYTES,
            record_json.len(),
        )?;
        transaction.execute(
            "INSERT INTO history_events
             (observed_at_unix_ms, source, record_kind, source_sequence, record_json, content_retained)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                observed_at,
                source,
                record_kind,
                sequence_bytes,
                record_json,
                i64::from(content_retained)
            ],
        )?;
        let id = transaction.last_insert_rowid();
        transaction.commit()?;
        Ok(id)
    }
}

fn migrate(connection: &mut Connection) -> Result<(), HistoryError> {
    let version: i64 = connection.pragma_query_value(None, "user_version", |row| row.get(0))?;
    if version < 0 || version > i64::from(HISTORY_SCHEMA_VERSION) {
        return Err(HistoryError::UnsupportedSchema(
            u32::try_from(version).unwrap_or(u32::MAX),
        ));
    }
    if version == i64::from(HISTORY_SCHEMA_VERSION) {
        return Ok(());
    }

    let transaction =
        connection.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
    transaction.execute_batch(
        "CREATE TABLE history_events (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            observed_at_unix_ms INTEGER NOT NULL CHECK(observed_at_unix_ms >= 0),
            source TEXT NOT NULL CHECK(length(source) <= 64),
            record_kind TEXT NOT NULL CHECK(length(record_kind) <= 64),
            source_sequence BLOB CHECK(source_sequence IS NULL OR (typeof(source_sequence) = 'blob' AND length(source_sequence) = 8)),
            record_json TEXT NOT NULL CHECK(length(CAST(record_json AS BLOB)) <= 65536),
            content_retained INTEGER NOT NULL CHECK(content_retained IN (0, 1))
        );
        CREATE INDEX history_events_observed_at ON history_events(observed_at_unix_ms, id);
        PRAGMA user_version = 1;",
    )?;
    transaction.commit()?;
    Ok(())
}

fn prune_history(
    transaction: &Transaction<'_>,
    maximum_rows: usize,
    maximum_payload_bytes: usize,
    incoming_bytes: usize,
) -> Result<(), HistoryError> {
    if maximum_rows == 0 || incoming_bytes > maximum_payload_bytes {
        return Err(HistoryError::DatabaseExceedsBound);
    }
    let maximum_rows =
        i64::try_from(maximum_rows - 1).map_err(|_| HistoryError::DatabaseExceedsBound)?;
    let prior_payload_limit = maximum_payload_bytes - incoming_bytes;
    let prior_payload_limit =
        i64::try_from(prior_payload_limit).map_err(|_| HistoryError::DatabaseExceedsBound)?;
    transaction.execute(
        "WITH newest AS (
             SELECT id,
                    ROW_NUMBER() OVER (ORDER BY id DESC) AS newest_rank,
                    SUM(length(CAST(record_json AS BLOB))) OVER (
                        ORDER BY id DESC ROWS BETWEEN UNBOUNDED PRECEDING AND CURRENT ROW
                    ) AS newest_payload_bytes
             FROM history_events
         )
         DELETE FROM history_events
         WHERE id IN (
             SELECT id FROM newest
             WHERE newest_rank > ?1 OR newest_payload_bytes > ?2
         )",
        params![maximum_rows, prior_payload_limit],
    )?;
    Ok(())
}

fn sanitize_broker_update(update: &mut BrokerUpdate, retain_content: bool) {
    let sanitize = |value: &mut Option<String>| {
        if retain_content {
            if let Some(text) = value {
                *text = crate::log::redact_sensitive_text(text);
            }
        } else {
            *value = None;
        }
    };
    match update {
        BrokerUpdate::UpsertSession(session) => sanitize(&mut session.display_name),
        BrokerUpdate::PatchSessionState(_) => {}
        BrokerUpdate::UpsertTurn(turn) => {
            sanitize(&mut turn.summary);
            sanitize(&mut turn.plan);
        }
        BrokerUpdate::UpsertToolItem(tool) => {
            sanitize(&mut tool.display_name);
            sanitize(&mut tool.sanitized_target);
        }
        BrokerUpdate::UpsertPendingRequest(_) => {}
        BrokerUpdate::UpsertRepository(repository) => {
            for field in [
                &mut repository.host,
                &mut repository.owner,
                &mut repository.name,
                &mut repository.branch,
                &mut repository.head_sha,
            ] {
                sanitize(field);
            }
        }
        BrokerUpdate::UpsertIntegration(_) => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::broker::reducer::BrokerUpdate;
    use crate::broker::types::{
        EventCorrelation, EventSequence, SchemaVersion, Session, SessionActivity, SessionLifecycle,
        SourceKind, SourceScopedId, WaitingState,
    };
    use codex_hook_contract::{CodexHookEvent, CodexHookObservation};
    use std::sync::atomic::{AtomicU64, Ordering};

    fn test_path(label: &str) -> std::path::PathBuf {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let directory = std::env::current_dir()
            .unwrap()
            .join("target")
            .join("history-tests")
            .join(format!(
                "{label}-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
        std::fs::create_dir_all(&directory).unwrap();
        directory.join("history.sqlite3")
    }

    fn session() -> Session {
        Session {
            schema_version: SchemaVersion::current(),
            identity: SourceScopedId::new(
                SourceKind::CodexCliObserver,
                crate::broker::types::OpaqueId::new("session-1").unwrap(),
            ),
            thread_id: None,
            agent_id: None,
            parent_agent_id: None,
            repository_id: None,
            worktree_id: None,
            display_name: Some("Project C:\\Users\\Alice\\private".into()),
            model: Some("codex".into()),
            lifecycle: SessionLifecycle::Active,
            activity: SessionActivity::Working,
            waiting: WaitingState::Unknown,
            connection: crate::broker::types::ConnectionHealth::Healthy,
            started_at_unix_ms: Some(100),
            last_seen_at_unix_ms: Some(101),
        }
    }

    fn event(payload: BrokerUpdate, sequence: u64) -> EventEnvelope<BrokerUpdate> {
        EventEnvelope {
            schema_version: SchemaVersion::current(),
            sequence: EventSequence::new(sequence).unwrap(),
            source: SourceKind::CodexCliObserver,
            source_event_id: None,
            observed_at_unix_ms: 1_000 + sequence,
            correlation: EventCorrelation::default(),
            deduplication_key: None,
            payload,
        }
    }

    #[test]
    fn fresh_database_migrates_transactionally_and_bounds_page_count() {
        let path = test_path("migration");
        let store = HistoryStore::open(&path).unwrap();
        assert_eq!(store.len().unwrap(), 0);
        let version: u32 = store
            .connection
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .unwrap();
        assert_eq!(version, HISTORY_SCHEMA_VERSION);
        let page_size: i64 = store
            .connection
            .pragma_query_value(None, "page_size", |row| row.get(0))
            .unwrap();
        let max_pages: i64 = store
            .connection
            .pragma_query_value(None, "max_page_count", |row| row.get(0))
            .unwrap();
        assert!(max_pages.saturating_mul(page_size) <= MAX_HISTORY_DATABASE_BYTES as i64);
        drop(store);
        std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }

    #[test]
    fn history_persists_only_the_validated_hook_discriminator() {
        let path = test_path("hook-event");
        let mut store = HistoryStore::open(&path).unwrap();
        let id = store
            .record_hook_observation(CodexHookObservation::new(CodexHookEvent::Stop), 42)
            .unwrap();
        let records = store.records_after(0, 1).unwrap();
        assert_eq!(id, records[0].id);
        assert_eq!(records[0].source, "codex_cli_observer");
        assert_eq!(records[0].record_kind, HOOK_OBSERVATION_KIND);
        assert_eq!(
            records[0].record_json,
            r#"{"wire_version":1,"event":"Stop"}"#
        );
        assert!(!records[0].content_retained);
        assert_eq!(store.len().unwrap(), 1);
        drop(store);
        let mut store = HistoryStore::open(&path).unwrap();
        assert_eq!(store.records_after(0, 1).unwrap(), records);
        assert!(matches!(
            store.record_hook_observation(
                CodexHookObservation {
                    wire_version: codex_hook_contract::WIRE_VERSION + 1,
                    event: CodexHookEvent::Stop,
                },
                43
            ),
            Err(HistoryError::InvalidHookWireVersion(_))
        ));
        drop(store);
        std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }

    #[test]
    fn broker_free_text_is_omitted_by_default_and_redacted_when_opted_in() {
        let path = test_path("content-policy");
        let mut store = HistoryStore::open(&path).unwrap();
        let session_event = event(BrokerUpdate::UpsertSession(session()), 7);
        store.record_broker_event(&session_event, false).unwrap();
        store.record_broker_event(&session_event, true).unwrap();
        let turn = crate::broker::types::Turn {
            schema_version: SchemaVersion::current(),
            identity: SourceScopedId::new(
                SourceKind::CodexCliObserver,
                crate::broker::types::OpaqueId::new("turn-1").unwrap(),
            ),
            session_id: SourceScopedId::new(
                SourceKind::CodexCliObserver,
                crate::broker::types::OpaqueId::new("session-1").unwrap(),
            ),
            thread_id: None,
            agent_id: None,
            lifecycle: crate::broker::types::TurnLifecycle::Completed,
            outcome: crate::broker::types::TurnOutcome::Success,
            started_at_unix_ms: Some(100),
            ended_at_unix_ms: Some(200),
            summary: Some("API key: sk-ant-private-value".into()),
            plan: Some(r"read C:\Users\Alice\secret\plan.md".into()),
        };
        store
            .record_broker_event(&event(BrokerUpdate::UpsertTurn(turn.clone()), 8), false)
            .unwrap();
        store
            .record_broker_event(&event(BrokerUpdate::UpsertTurn(turn), 9), true)
            .unwrap();
        let rows = store.records_after(0, 10).unwrap();
        let redacted: serde_json::Value = serde_json::from_str(&rows[0].record_json).unwrap();
        let retained: serde_json::Value = serde_json::from_str(&rows[1].record_json).unwrap();
        assert_eq!(
            redacted["payload"]["value"]["displayName"],
            serde_json::Value::Null
        );
        assert_eq!(
            retained["payload"]["value"]["displayName"],
            "Project [REDACTED]"
        );
        assert_eq!(rows[0].source_sequence, Some(7));
        assert!(!rows[0].content_retained);
        assert!(rows[1].content_retained);
        assert!(!rows[1].record_json.contains("Alice"));
        let turn_without_content: serde_json::Value =
            serde_json::from_str(&rows[2].record_json).unwrap();
        let turn_with_content: serde_json::Value =
            serde_json::from_str(&rows[3].record_json).unwrap();
        assert_eq!(
            turn_without_content["payload"]["value"]["summary"],
            serde_json::Value::Null
        );
        assert_eq!(
            turn_without_content["payload"]["value"]["plan"],
            serde_json::Value::Null
        );
        assert_eq!(
            turn_with_content["payload"]["value"]["summary"],
            "API key: [REDACTED]"
        );
        assert!(!turn_with_content.to_string().contains("Alice"));
        assert!(!turn_with_content
            .to_string()
            .contains("sk-ant-private-value"));
        drop(store);
        std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }

    #[test]
    fn row_and_payload_bounds_prune_oldest_records_before_append() {
        let path = test_path("bounded-rows");
        let mut connection = Connection::open(&path).unwrap();
        migrate(&mut connection).unwrap();
        for value in 0..5 {
            let transaction = connection.transaction().unwrap();
            prune_history(&transaction, 3, MAX_HISTORY_PAYLOAD_BYTES, 2).unwrap();
            transaction
                .execute(
                    "INSERT INTO history_events
                     (observed_at_unix_ms, source, record_kind, source_sequence, record_json, content_retained)
                     VALUES (?1, 'windows', 'test', NULL, ?2, 0)",
                    params![value, "{}"],
                )
                .unwrap();
            transaction.commit().unwrap();
        }
        let count: i64 = connection
            .query_row("SELECT COUNT(*) FROM history_events", [], |row| row.get(0))
            .unwrap();
        let first_id: i64 = connection
            .query_row("SELECT MIN(id) FROM history_events", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 3);
        assert_eq!(first_id, 3);

        let transaction = connection.transaction().unwrap();
        prune_history(&transaction, 10, 7, 2).unwrap();
        transaction.commit().unwrap();
        let remaining_payload: i64 = connection
            .query_row(
                "SELECT COALESCE(SUM(length(CAST(record_json AS BLOB))), 0) FROM history_events",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert!(remaining_payload <= 5);
        drop(connection);
        std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }

    #[test]
    fn oversized_records_future_schemas_and_invalid_cursors_fail_closed() {
        let path = test_path("failure-bounds");
        let mut store = HistoryStore::open(&path).unwrap();
        let oversized = "x".repeat(MAX_HISTORY_RECORD_BYTES + 1);
        assert!(matches!(
            store.insert_record(1, "windows", "test", None, &oversized, false),
            Err(HistoryError::RecordTooLarge { .. })
        ));
        assert!(matches!(
            store.records_after(-1, 1),
            Err(HistoryError::InvalidCursor)
        ));
        drop(store);
        let connection = Connection::open(&path).unwrap();
        connection
            .pragma_update(None, "user_version", HISTORY_SCHEMA_VERSION + 1)
            .unwrap();
        drop(connection);
        assert!(matches!(
            HistoryStore::open(&path),
            Err(HistoryError::UnsupportedSchema(_))
        ));
        std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }
}
