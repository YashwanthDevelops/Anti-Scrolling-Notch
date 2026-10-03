//! Thread-safe access to the backend broker stream used by the Tauri bridge.

use std::sync::Mutex;

use super::reducer::BrokerUpdate;
use super::stream::{ApplyReceipt, BrokerStream, BrokerSyncResponse, StreamError};
use super::types::OpaqueId;

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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::broker::types::{
        ConfigurationState, ConnectionHealth, Integration, IntegrationProvider, SchemaVersion,
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
}
