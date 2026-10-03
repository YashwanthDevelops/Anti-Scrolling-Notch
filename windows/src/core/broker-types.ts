/** The existing Rust broker IPC projection; these are normalized app records, not Codex wire events. */

export type BrokerSource = "codex_cli_observer" | "local_git" | "github" | "windows" | "integration";

export interface SourceScopedId {
  source: BrokerSource;
  id: string;
}

export interface BrokerSession {
  schemaVersion: 1;
  identity: SourceScopedId;
  lifecycle: string;
  activity: string;
  waiting: string;
  connection: string;
  [key: string]: unknown;
}

export interface BrokerPendingRequest {
  schemaVersion: 1;
  identity: SourceScopedId;
  lifecycle: string;
  [key: string]: unknown;
}

export interface BrokerIntegration {
  schemaVersion: 1;
  identity: string;
  [key: string]: unknown;
}

export interface BrokerRecord {
  schemaVersion: 1;
  identity: SourceScopedId;
  [key: string]: unknown;
}

export interface BrokerSnapshot {
  schemaVersion: 1;
  sequence: number;
  sessions: BrokerSession[];
  turns: BrokerRecord[];
  toolItems: BrokerRecord[];
  pendingRequests: BrokerPendingRequest[];
  repositories: BrokerRecord[];
  integrations: BrokerIntegration[];
}

export interface BrokerEventEnvelope<T> {
  schemaVersion: 1;
  sequence: number;
  source: BrokerSource;
  sourceEventId: string | null;
  observedAtUnixMs: number;
  correlation: Record<string, SourceScopedId | null>;
  deduplicationKey: string | null;
  payload: T;
}

export type BrokerUpdate =
  | { kind: "upsert_session"; value: BrokerSession }
  | {
      kind: "patch_session_state";
      value: {
        identity: SourceScopedId;
        lifecycle: string | null;
        activity: string | null;
        waiting: string | null;
        connection: string | null;
      };
    }
  | { kind: "upsert_turn"; value: BrokerRecord }
  | { kind: "upsert_tool_item"; value: BrokerRecord }
  | { kind: "upsert_pending_request"; value: BrokerPendingRequest }
  | { kind: "upsert_repository"; value: BrokerRecord }
  | { kind: "upsert_integration"; value: BrokerIntegration };

export type BrokerSyncResponse =
  | { kind: "snapshot"; snapshot: BrokerSnapshot }
  | {
      kind: "replay";
      afterSequence: number;
      throughSequence: number;
      events: BrokerEventEnvelope<BrokerUpdate>[];
    };

/** A typed read intent; it cannot mutate backend state or enable a capability. */
export interface BrokerSyncIntent {
  kind: "synchronize";
  afterSequence: number;
}

export const EMPTY_BROKER_SNAPSHOT: BrokerSnapshot = {
  schemaVersion: 1,
  sequence: 0,
  sessions: [],
  turns: [],
  toolItems: [],
  pendingRequests: [],
  repositories: [],
  integrations: [],
};
