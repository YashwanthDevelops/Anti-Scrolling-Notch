import { Bridge, onEvent } from "./bridge";
import type {
  BrokerEventEnvelope,
  BrokerIntegration,
  BrokerPendingRequest,
  BrokerSession,
  BrokerSnapshot,
  BrokerSyncIntent,
  BrokerSyncResponse,
  BrokerUpdate,
  SourceScopedId,
} from "./broker-types.js";
import { EMPTY_BROKER_SNAPSHOT } from "./broker-types.js";

export type BrokerViewStatus = "idle" | "syncing" | "ready" | "error";

export interface BrokerSyncTransport {
  synchronize(intent: BrokerSyncIntent): Promise<BrokerSyncResponse>;
  listen(handler: (response: BrokerSyncResponse) => void): Promise<() => void>;
}

type DeepReadonly<T> = T extends (infer Item)[]
  ? readonly DeepReadonly<Item>[]
  : T extends object
    ? { readonly [Key in keyof T]: DeepReadonly<T[Key]> }
    : T;

type Subscriber = () => void;

const tauriTransport: BrokerSyncTransport = {
  synchronize: (intent) => Bridge.syncBroker(intent),
  listen: (handler) => onEvent<BrokerSyncResponse>("broker-sync", handler),
};

/**
 * A read-only frontend projection of Rust broker state. Backend snapshots are
 * authoritative; replay updates are accepted only in contiguous sequence.
 */
export class BrokerViewStore {
  private snapshotValue = freezeSnapshot(copySnapshot(EMPTY_BROKER_SNAPSHOT));
  private statusValue: BrokerViewStatus = "idle";
  private errorValue: Error | null = null;
  private readonly subscribers = new Set<Subscriber>();
  private unlisten: (() => void) | null = null;
  private startPromise: Promise<void> | null = null;
  private updateQueue: Promise<void> = Promise.resolve();

  constructor(private readonly transport: BrokerSyncTransport = tauriTransport) {}

  get snapshot(): DeepReadonly<BrokerSnapshot> {
    return this.snapshotValue;
  }

  get status(): BrokerViewStatus {
    return this.statusValue;
  }

  get error(): Error | null {
    return this.errorValue;
  }

  subscribe(subscriber: Subscriber): () => void {
    this.subscribers.add(subscriber);
    return () => this.subscribers.delete(subscriber);
  }

  /** Subscribe first so changes racing the initial snapshot can be replayed. */
  start(): Promise<void> {
    if (this.startPromise) return this.startPromise;
    if (this.unlisten) return this.synchronize(0);
    this.setStatus("syncing", null);
    this.startPromise = (async () => {
      try {
        this.unlisten = await this.transport.listen((response) => {
          void this.enqueue(response);
        });
        await this.synchronize(0);
      } catch (error) {
        this.fail(error);
      } finally {
        this.startPromise = null;
      }
    })();
    return this.startPromise;
  }

  stop(): void {
    this.unlisten?.();
    this.unlisten = null;
  }

  /** Retry the current cursor or explicitly request a full initial snapshot. */
  async synchronize(afterSequence = this.snapshotValue.sequence): Promise<void> {
    this.setStatus("syncing", null);
    try {
      const response = await this.transport.synchronize({ kind: "synchronize", afterSequence });
      await this.enqueue(response);
      if (this.statusValue !== "error") this.setStatus("ready", null);
    } catch (error) {
      this.fail(error);
    }
  }

  private enqueue(response: BrokerSyncResponse): Promise<void> {
    this.updateQueue = this.updateQueue
      .then(() => this.applyResponse(response))
      .catch((error: unknown) => this.fail(error));
    return this.updateQueue;
  }

  private async applyResponse(response: BrokerSyncResponse): Promise<void> {
    if (response.kind === "snapshot") {
      this.applySnapshot(response.snapshot);
      return;
    }
    if (response.kind !== "replay") {
      await this.recoverSnapshot();
      return;
    }

    const current = this.snapshotValue.sequence;
    if (!isSequence(response.afterSequence) || !isSequence(response.throughSequence)) {
      await this.recoverSnapshot();
      return;
    }
    if (response.throughSequence < current) return; // A delayed event is stale.
    if (response.afterSequence > current) {
      await this.recoverSnapshot();
      return;
    }

    let next = this.snapshotValue;
    let sequence = current;
    for (const event of response.events) {
      if (!isEnvelope(event) || event.sequence <= sequence) continue;
      if (event.sequence !== sequence + 1 || event.schemaVersion !== 1) {
        await this.recoverSnapshot();
        return;
      }
      try {
        next = applyUpdate(next, event.payload);
      } catch {
        await this.recoverSnapshot();
        return;
      }
      sequence = event.sequence;
    }

    if (sequence !== response.throughSequence) {
      if (sequence === current && response.throughSequence === current) return;
      await this.recoverSnapshot();
      return;
    }

    this.snapshotValue = freezeSnapshot({ ...next, sequence });
    this.setStatus("ready", null);
  }

  private async recoverSnapshot(): Promise<void> {
    const response = await this.transport.synchronize({ kind: "synchronize", afterSequence: 0 });
    if (response.kind !== "snapshot") {
      throw new Error("backend did not return a snapshot for a full synchronization request");
    }
    this.applySnapshot(response.snapshot);
  }

  private applySnapshot(snapshot: BrokerSnapshot): void {
    if (!isSnapshot(snapshot)) {
      throw new Error("backend returned an unsupported broker snapshot");
    }
    if (snapshot.sequence < this.snapshotValue.sequence) return;
    this.snapshotValue = freezeSnapshot(copySnapshot(snapshot));
    this.setStatus("ready", null);
  }

  private setStatus(status: BrokerViewStatus, error: Error | null): void {
    this.statusValue = status;
    this.errorValue = error;
    this.notify();
  }

  private fail(error: unknown): void {
    const failure = error instanceof Error ? error : new Error(String(error));
    this.setStatus("error", failure);
  }

  private notify(): void {
    for (const subscriber of this.subscribers) subscriber();
  }
}

function isSequence(value: number): boolean {
  return Number.isSafeInteger(value) && value >= 0;
}

function isEnvelope(value: BrokerEventEnvelope<BrokerUpdate>): boolean {
  return Number.isSafeInteger(value.sequence) && value.sequence > 0;
}

function isSnapshot(value: BrokerSnapshot): boolean {
  return value.schemaVersion === 1
    && isSequence(value.sequence)
    && Array.isArray(value.sessions)
    && Array.isArray(value.turns)
    && Array.isArray(value.toolItems)
    && Array.isArray(value.pendingRequests)
    && Array.isArray(value.repositories)
    && Array.isArray(value.integrations);
}

function copySnapshot(snapshot: BrokerSnapshot): BrokerSnapshot {
  return {
    ...snapshot,
    sessions: [...snapshot.sessions],
    turns: [...snapshot.turns],
    toolItems: [...snapshot.toolItems],
    pendingRequests: [...snapshot.pendingRequests],
    repositories: [...snapshot.repositories],
    integrations: [...snapshot.integrations],
  };
}

function freezeSnapshot(snapshot: BrokerSnapshot): BrokerSnapshot {
  return deepFreeze(snapshot);
}

function deepFreeze<T>(value: T): T {
  if (value && typeof value === "object" && !Object.isFrozen(value)) {
    for (const nested of Object.values(value as Record<string, unknown>)) deepFreeze(nested);
    Object.freeze(value);
  }
  return value;
}

function sourceScopedKey(identity: SourceScopedId): string {
  // The Rust ID validator rejects control characters, including this separator.
  return `${identity.source}\u0000${identity.id}`;
}

function upsert<T extends { identity: SourceScopedId }>(items: T[], next: T): T[] {
  const key = sourceScopedKey(next.identity);
  const index = items.findIndex((item) => sourceScopedKey(item.identity) === key);
  if (index < 0) return [...items, next];
  const copy = [...items];
  copy[index] = next;
  return copy;
}

function upsertIntegration(items: BrokerIntegration[], next: BrokerIntegration): BrokerIntegration[] {
  const index = items.findIndex((item) => item.identity === next.identity);
  if (index < 0) return [...items, next];
  const copy = [...items];
  copy[index] = next;
  return copy;
}

function emptySession(identity: SourceScopedId): BrokerSession {
  return {
    schemaVersion: 1,
    identity,
    threadId: null,
    agentId: null,
    parentAgentId: null,
    repositoryId: null,
    worktreeId: null,
    displayName: null,
    model: null,
    lifecycle: "unknown",
    activity: "unknown",
    waiting: "unknown",
    connection: "unknown",
    startedAtUnixMs: null,
    lastSeenAtUnixMs: null,
  };
}

function applyUpdate(snapshot: BrokerSnapshot, update: BrokerUpdate): BrokerSnapshot {
  switch (update.kind) {
    case "upsert_session":
      return { ...snapshot, sessions: upsert(snapshot.sessions, update.value) };
    case "patch_session_state": {
      const patch = update.value;
      const key = sourceScopedKey(patch.identity);
      const existing = snapshot.sessions.find((session) => sourceScopedKey(session.identity) === key);
      if (!existing && [patch.lifecycle, patch.activity, patch.waiting, patch.connection].every((x) => x == null)) {
        return snapshot;
      }
      const next: BrokerSession = { ...(existing ?? emptySession(patch.identity)) };
      if (patch.lifecycle != null) next.lifecycle = patch.lifecycle;
      if (patch.activity != null) next.activity = patch.activity;
      if (patch.waiting != null) next.waiting = patch.waiting;
      if (patch.connection != null) next.connection = patch.connection;
      return { ...snapshot, sessions: upsert(snapshot.sessions, next) };
    }
    case "upsert_turn":
      return { ...snapshot, turns: upsert(snapshot.turns, update.value) };
    case "upsert_tool_item":
      return { ...snapshot, toolItems: upsert(snapshot.toolItems, update.value) };
    case "upsert_pending_request":
      return applyPendingRequest(snapshot, update.value);
    case "upsert_repository":
      return { ...snapshot, repositories: upsert(snapshot.repositories, update.value) };
    case "upsert_integration":
      return { ...snapshot, integrations: upsertIntegration(snapshot.integrations, update.value) };
    default:
      throw new Error("backend sent an unsupported broker update");
  }
}

function applyPendingRequest(
  snapshot: BrokerSnapshot,
  next: BrokerPendingRequest,
): BrokerSnapshot {
  const key = sourceScopedKey(next.identity);
  const previous = snapshot.pendingRequests.find((request) => sourceScopedKey(request.identity) === key);
  if (["resolved", "expired", "cancelled"].includes(next.lifecycle)) {
    if (!previous) return snapshot;
    const expected = { ...previous, lifecycle: next.lifecycle };
    return deepEqual(expected, next)
      ? { ...snapshot, pendingRequests: snapshot.pendingRequests.filter((request) => request !== previous) }
      : snapshot;
  }
  return { ...snapshot, pendingRequests: upsert(snapshot.pendingRequests, next) };
}

function deepEqual(left: unknown, right: unknown): boolean {
  if (Object.is(left, right)) return true;
  if (!left || !right || typeof left !== "object" || typeof right !== "object") return false;
  if (Array.isArray(left) || Array.isArray(right)) {
    return Array.isArray(left) && Array.isArray(right)
      && left.length === right.length
      && left.every((item, index) => deepEqual(item, right[index]));
  }
  const leftRecord = left as Record<string, unknown>;
  const rightRecord = right as Record<string, unknown>;
  const leftKeys = Object.keys(leftRecord).sort();
  const rightKeys = Object.keys(rightRecord).sort();
  return leftKeys.length === rightKeys.length
    && leftKeys.every((key, index) => key === rightKeys[index] && deepEqual(leftRecord[key], rightRecord[key]));
}

export const BrokerView = new BrokerViewStore();
