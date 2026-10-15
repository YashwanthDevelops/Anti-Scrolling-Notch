//! Thread-safe access to the backend broker stream used by the Tauri bridge.

use std::sync::Mutex;

use super::reducer::BrokerUpdate;
use super::stream::{ApplyReceipt, BrokerSnapshot, BrokerStream, BrokerSyncResponse, StreamError};
use super::types::{
    EventCorrelation, EventEnvelope, EventSequence, OpaqueId, SchemaVersion, SourceKind,
};

#[derive(Debug)]
pub enum BrokerServiceError {
    LockPoisoned,
    Stream(StreamError),
}

impl std::fmt::Display for BrokerServiceError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::LockPoisoned => formatter.write_str("backend broker state is unavailable"),
            Self::Stream(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for BrokerServiceError {}

impl From<StreamError> for BrokerServiceError {
    fn from(error: StreamError) -> Self {
        Self::Stream(error)
    }
}

/// Owns the one process-local broker stream shared by commands and adapters.
#[derive(Default)]
pub struct BrokerService {
    stream: Mutex<BrokerStream>,
}

/// Result of one serialized multi-update observation. `events` contains only
/// changed broker envelopes with the real local sequence numbers allocated by
/// the stream.
#[derive(Clone, Debug)]
pub struct BrokerBatchResult {
    pub receipts: Vec<ApplyReceipt>,
    pub events: Vec<EventEnvelope<BrokerUpdate>>,
    pub notification: Option<BrokerSyncResponse>,
}

impl BrokerService {
    /// Return a complete snapshot or a contiguous replay from the current stream.
    pub fn synchronize_after(
        &self,
        after_sequence: u64,
    ) -> Result<BrokerSyncResponse, BrokerServiceError> {
        let stream = self
            .stream
            .lock()
            .map_err(|_| BrokerServiceError::LockPoisoned)?;
        Ok(stream.synchronize_after(after_sequence))
    }

    /// Install the persisted event high-water during startup recovery, before
    /// producers publish new live events.
    pub(crate) fn advance_after_recovery(&self, high_water: u64) -> Result<(), BrokerServiceError> {
        let mut stream = self
            .stream
            .lock()
            .map_err(|_| BrokerServiceError::LockPoisoned)?;
        stream.advance_after_recovery(high_water);
        Ok(())
    }

    /// Apply an already-normalized backend update and return the response that
    /// should be published to view-store listeners, when state changed.
    pub fn apply(
        &self,
        update: BrokerUpdate,
        observed_at_unix_ms: u64,
        source_event_id: Option<OpaqueId>,
        deduplication_key: Option<OpaqueId>,
    ) -> Result<(ApplyReceipt, Option<BrokerSyncResponse>), BrokerServiceError> {
        let mut stream = self
            .stream
            .lock()
            .map_err(|_| BrokerServiceError::LockPoisoned)?;
        let before = stream.sequence();
        let receipt = stream.apply_with_source_ids(
            update,
            observed_at_unix_ms,
            source_event_id,
            deduplication_key,
        )?;
        let notification = (receipt.sequence > before).then(|| stream.synchronize_after(before));
        Ok((receipt, notification))
    }

    /// Apply the normalized updates from one source observation under one
    /// broker lock. If any sequence allocation fails, restore the original
    /// stream so callers never expose a partially applied observation.
    pub fn apply_batch(
        &self,
        updates: Vec<BrokerUpdate>,
        observed_at_unix_ms: u64,
    ) -> Result<BrokerBatchResult, BrokerServiceError> {
        self.apply_batch_with(observed_at_unix_ms, |_| updates)
    }

    /// Snapshot, normalize, and apply one observation while holding the broker
    /// lock. The closure receives read-only current state and must return only
    /// backend-normalized updates.
    pub fn apply_batch_with(
        &self,
        observed_at_unix_ms: u64,
        normalize: impl FnOnce(&BrokerSnapshot) -> Vec<BrokerUpdate>,
    ) -> Result<BrokerBatchResult, BrokerServiceError> {
        let mut stream = self
            .stream
            .lock()
            .map_err(|_| BrokerServiceError::LockPoisoned)?;
        let original = stream.clone();
        let before = stream.sequence();
        let snapshot = stream.snapshot();
        let updates = normalize(&snapshot);
        let mut receipts = Vec::with_capacity(updates.len());
        let mut events = Vec::with_capacity(updates.len());
        for update in updates {
            let source = update_source(&update);
            let receipt =
                match stream.apply_with_source_ids(update.clone(), observed_at_unix_ms, None, None)
                {
                    Ok(receipt) => receipt,
                    Err(error) => {
                        *stream = original;
                        return Err(error.into());
                    }
                };
            if receipt.outcome == super::reducer::ApplyOutcome::Changed {
                let sequence = EventSequence::new(receipt.sequence)
                    .expect("changed updates have a non-zero broker sequence");
                events.push(EventEnvelope {
                    schema_version: SchemaVersion::current(),
                    sequence,
                    source,
                    source_event_id: None,
                    observed_at_unix_ms,
                    correlation: EventCorrelation::default(),
                    deduplication_key: None,
                    payload: update,
                });
            }
            receipts.push(receipt);
        }
        let changed = stream.sequence() > before;
        let notification = changed.then(|| stream.synchronize_after(before));
        Ok(BrokerBatchResult {
            receipts,
            events,
            notification,
        })
    }
}

fn update_source(update: &BrokerUpdate) -> SourceKind {
    match update {
        BrokerUpdate::UpsertSession(value) => value.identity.source,
        BrokerUpdate::PatchSessionState(value) => value.identity.source,
        BrokerUpdate::UpsertTurn(value) => value.identity.source,
        BrokerUpdate::UpsertToolItem(value) => value.identity.source,
        BrokerUpdate::UpsertPendingRequest(value) => value.identity.source,
        BrokerUpdate::UpsertRepository(value) => value.identity.source,
        BrokerUpdate::UpsertIntegration(_) => SourceKind::Integration,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::broker::types::{
        ConfigurationState, ConnectionHealth, Integration, IntegrationProvider, OpaqueId,
        SchemaVersion, Session, SessionActivity, SessionLifecycle, SourceKind, SourceScopedId,
        Turn, TurnLifecycle, TurnOutcome, WaitingState,
    };

    fn integration(id: &str, data_revision: u64) -> Integration {
        Integration {
            schema_version: SchemaVersion::current(),
            identity: OpaqueId::new(id).unwrap(),
            provider: IntegrationProvider::GitHub,
            configuration: ConfigurationState::Configured,
            connection: ConnectionHealth::Healthy,
            last_success_at_unix_ms: Some(100),
            data_revision: Some(data_revision),
            retry_at_unix_ms: None,
            last_error: None,
            unread_event_ids: Vec::new(),
        }
    }

    fn session() -> Session {
        Session {
            schema_version: SchemaVersion::current(),
            identity: SourceScopedId::new(
                SourceKind::CodexCliObserver,
                OpaqueId::new("session-batch").unwrap(),
            ),
            thread_id: None,
            agent_id: None,
            parent_agent_id: None,
            repository_id: None,
            worktree_id: None,
            display_name: None,
            model: None,
            lifecycle: SessionLifecycle::Active,
            activity: SessionActivity::Working,
            waiting: WaitingState::Unknown,
            connection: ConnectionHealth::Healthy,
            started_at_unix_ms: Some(100),
            last_seen_at_unix_ms: Some(100),
        }
    }

    #[test]
    fn initial_sync_is_a_complete_snapshot() {
        let service = BrokerService::default();
        assert!(matches!(
            service.synchronize_after(0).unwrap(),
            BrokerSyncResponse::Snapshot { snapshot } if snapshot.sequence == 0 && snapshot.integrations.is_empty()
        ));
    }

    #[test]
    fn accepted_updates_publish_contiguous_replay_and_duplicates_are_noops() {
        let service = BrokerService::default();
        let (first, initial) = service
            .apply(
                BrokerUpdate::UpsertIntegration(integration("github", 1)),
                100,
                Some(OpaqueId::new("event-1").unwrap()),
                None,
            )
            .unwrap();
        assert_eq!(first.sequence, 1);
        assert!(matches!(initial, Some(BrokerSyncResponse::Snapshot { .. })));

        let update = BrokerUpdate::UpsertIntegration(integration("github", 2));
        let (second, replay) = service
            .apply(
                update.clone(),
                200,
                Some(OpaqueId::new("event-2").unwrap()),
                None,
            )
            .unwrap();
        assert_eq!(second.sequence, 2);
        assert!(matches!(
            replay,
            Some(BrokerSyncResponse::Replay {
                after_sequence: 1,
                through_sequence: 2,
                events,
            }) if events.len() == 1 && events[0].sequence.get() == 2
        ));

        let (duplicate, notification) = service
            .apply(update, 300, Some(OpaqueId::new("event-2").unwrap()), None)
            .unwrap();
        assert!(duplicate.deduplicated);
        assert_eq!(duplicate.sequence, 2);
        assert!(notification.is_none());
    }

    #[test]
    fn batch_updates_are_atomic_and_return_the_actual_changed_envelopes() {
        let service = BrokerService::default();
        let session = session();
        let turn = Turn {
            schema_version: SchemaVersion::current(),
            identity: SourceScopedId::new(
                SourceKind::CodexCliObserver,
                OpaqueId::new("turn-batch").unwrap(),
            ),
            session_id: session.identity.clone(),
            thread_id: None,
            agent_id: None,
            lifecycle: TurnLifecycle::Active,
            outcome: TurnOutcome::Unknown,
            started_at_unix_ms: Some(100),
            ended_at_unix_ms: None,
            summary: None,
            plan: None,
        };
        let batch = service
            .apply_batch(
                vec![
                    BrokerUpdate::UpsertSession(session),
                    BrokerUpdate::UpsertTurn(turn),
                ],
                100,
            )
            .unwrap();
        assert_eq!(batch.events.len(), 2);
        assert_eq!(batch.events[0].sequence.get(), 1);
        assert_eq!(batch.events[1].sequence.get(), 2);
        assert!(matches!(
            batch.notification,
            Some(BrokerSyncResponse::Snapshot { snapshot })
                if snapshot.sequence == 2 && snapshot.sessions.len() == 1 && snapshot.turns.len() == 1
        ));
    }

    #[test]
    fn failed_batch_restores_stream_without_partial_state_or_sequence() {
        let service = BrokerService::default();
        service
            .stream
            .lock()
            .unwrap()
            .set_sequence_for_test(u64::MAX - 1);
        let result = service.apply_batch(
            vec![
                BrokerUpdate::UpsertIntegration(integration("first", 1)),
                BrokerUpdate::UpsertIntegration(integration("second", 2)),
            ],
            100,
        );
        assert!(matches!(
            result,
            Err(BrokerServiceError::Stream(StreamError::SequenceExhausted))
        ));
        let stream = service.stream.lock().unwrap();
        assert_eq!(stream.sequence(), u64::MAX - 1);
        assert!(stream.state().integrations().next().is_none());
    }
}
