import assert from "node:assert/strict";
import { after, before, test } from "node:test";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { createServer } from "vite";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
let vite;
let BrokerViewStore;
let BridgeCallError;

before(async () => {
  vite = await createServer({
    root,
    configFile: false,
    server: { middlewareMode: true, watch: null, hmr: false },
    appType: "custom",
    optimizeDeps: { noDiscovery: true, include: [], holdUntilCrawlEnd: false },
  });
  ({ BrokerViewStore } = await vite.ssrLoadModule("/src/core/view-store.ts"));
  ({ BridgeCallError } = await vite.ssrLoadModule("/src/core/bridge.ts"));
});

after(async () => {
  await vite?.close();
});

function emptySnapshot(sequence = 0) {
  return {
    schemaVersion: 1,
    sequence,
    sessions: [],
    turns: [],
    toolItems: [],
    pendingRequests: [],
    repositories: [],
    integrations: [],
  };
}

function session(id, fields = {}) {
  return {
    schemaVersion: 1,
    identity: { source: "codex_cli_observer", id },
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
    ...fields,
  };
}

function pendingRequest(id, lifecycle = "pending", fields = {}) {
  return {
    schemaVersion: 1,
    identity: { source: "codex_cli_observer", id },
    sessionId: null,
    threadId: null,
    turnId: null,
    toolItemId: null,
    agentId: null,
    kind: "approval",
    lifecycle,
    createdAtUnixMs: 100,
    deadlineUnixMs: 1000,
    ...fields,
  };
}

function integration(id, connection = "healthy", fields = {}) {
  return {
    schemaVersion: 1,
    identity: id,
    provider: "github",
    configuration: "unknown",
    connection,
    lastSuccessAtUnixMs: null,
    dataRevision: null,
    retryAtUnixMs: null,
    lastError: null,
    unreadEventIds: [],
    ...fields,
  };
}

function envelope(sequence, payload) {
  return {
    schemaVersion: 1,
    sequence,
    source: "codex_cli_observer",
    sourceEventId: null,
    observedAtUnixMs: sequence * 100,
    correlation: {
      sessionId: null,
      threadId: null,
      turnId: null,
      toolItemId: null,
      agentId: null,
      repositoryId: null,
    },
    deduplicationKey: null,
    payload,
  };
}

class TestTransport {
  calls = [];
  order = [];
  handler = null;
  responses = [];

  async synchronize(intent) {
    this.order.push("sync");
    this.calls.push(intent);
    const response = this.responses.shift();
    if (response instanceof Error) throw response;
    if (!response) throw new Error("missing test response");
    return response;
  }

  async listen(handler) {
    this.order.push("listen");
    this.handler = handler;
    return () => { this.handler = null; };
  }

  emit(response) {
    this.handler?.(response);
  }
}

async function flushEvents() {
  await new Promise((resolve) => setTimeout(resolve, 0));
}

test("subscribes before requesting the initial backend snapshot", async () => {
  const transport = new TestTransport();
  transport.responses.push({ kind: "snapshot", snapshot: emptySnapshot() });
  const store = new BrokerViewStore(transport);

  await store.start();

  assert.deepEqual(transport.order, ["listen", "sync"]);
  assert.deepEqual(transport.calls, [{ kind: "synchronize", afterSequence: 0 }]);
  assert.equal(store.status, "ready");
  assert.equal(store.snapshot.sequence, 0);
  assert.ok(Object.isFrozen(store.snapshot));
  assert.ok(Object.isFrozen(store.snapshot.sessions));
});

test("applies only the contiguous replay and ignores delayed duplicate events", async () => {
  const transport = new TestTransport();
  transport.responses.push({ kind: "snapshot", snapshot: emptySnapshot(0) });
  const store = new BrokerViewStore(transport);
  await store.start();

  const first = session("session-a");
  const patch = {
    identity: first.identity,
    lifecycle: "active",
    activity: "working",
    waiting: null,
    connection: "healthy",
  };
  const replay = {
    kind: "replay",
    afterSequence: 0,
    throughSequence: 2,
    events: [
      envelope(1, { kind: "upsert_session", value: first }),
      envelope(2, { kind: "patch_session_state", value: patch }),
    ],
  };
  transport.emit(replay);
  await flushEvents();

  assert.equal(store.snapshot.sequence, 2);
  assert.equal(store.snapshot.sessions[0].lifecycle, "active");
  assert.equal(store.snapshot.sessions[0].activity, "working");
  assert.equal(store.snapshot.sessions[0].connection, "healthy");
  assert.ok(Object.isFrozen(store.snapshot.sessions[0]));
  assert.ok(Object.isFrozen(store.snapshot.sessions[0].identity));

  transport.emit({
    kind: "replay",
    afterSequence: 0,
    throughSequence: 2,
    events: replay.events,
  });
  await flushEvents();
  assert.equal(store.snapshot.sequence, 2);
  assert.equal(store.snapshot.sessions.length, 1);
});

test("resynchronizes when replay events arrive out of order", async () => {
  const transport = new TestTransport();
  transport.responses.push({ kind: "snapshot", snapshot: emptySnapshot() });
  const store = new BrokerViewStore(transport);
  await store.start();
  const recovered = emptySnapshot(2);
  recovered.sessions.push(session("latest"));
  transport.responses.push({ kind: "snapshot", snapshot: recovered });

  transport.emit({
    kind: "replay",
    afterSequence: 0,
    throughSequence: 2,
    events: [envelope(2, { kind: "upsert_session", value: session("latest") })],
  });
  await flushEvents();

  assert.deepEqual(transport.calls.at(-1), { kind: "synchronize", afterSequence: 0 });
  assert.equal(store.snapshot.sequence, 2);
  assert.equal(store.snapshot.sessions[0].identity.id, "latest");
  assert.equal(store.status, "ready");
});

test("resynchronizes with a full snapshot when the replay sequence has a gap", async () => {
  const transport = new TestTransport();
  transport.responses.push({ kind: "snapshot", snapshot: emptySnapshot(1) });
  const store = new BrokerViewStore(transport);
  await store.start();
  const recovered = emptySnapshot(3);
  recovered.sessions.push(session("latest"));
  transport.responses.push({ kind: "snapshot", snapshot: recovered });

  transport.emit({
    kind: "replay",
    afterSequence: 1,
    throughSequence: 3,
    events: [envelope(3, { kind: "upsert_session", value: session("latest") })],
  });
  await flushEvents();

  assert.deepEqual(transport.calls.at(-1), { kind: "synchronize", afterSequence: 0 });
  assert.equal(store.snapshot.sequence, 3);
  assert.equal(store.snapshot.sessions[0].identity.id, "latest");
  assert.equal(store.status, "ready");
});

test("terminal request replay removes only the exact matching active record", async () => {
  const transport = new TestTransport();
  const active = pendingRequest("request-a");
  const unrelated = pendingRequest("request-b", "pending", { deadlineUnixMs: 2000 });
  const initial = emptySnapshot(1);
  initial.pendingRequests.push(active, unrelated);
  transport.responses.push({ kind: "snapshot", snapshot: initial });
  const store = new BrokerViewStore(transport);
  await store.start();

  transport.emit({
    kind: "replay",
    afterSequence: 1,
    throughSequence: 2,
    events: [envelope(2, {
      kind: "upsert_pending_request",
      value: pendingRequest("request-a", "resolved"),
    })],
  });
  await flushEvents();
  assert.equal(store.snapshot.sequence, 2);
  assert.deepEqual(store.snapshot.pendingRequests, [unrelated]);
});

test("integration health is recovered from snapshots and applied through sequenced replay", async () => {
  const transport = new TestTransport();
  const initial = emptySnapshot(3);
  initial.integrations.push(integration("integration_github"));
  transport.responses.push({ kind: "snapshot", snapshot: initial });
  const store = new BrokerViewStore(transport);
  await store.start();

  transport.emit({
    kind: "replay",
    afterSequence: 3,
    throughSequence: 4,
    events: [envelope(4, {
      kind: "upsert_integration",
      value: integration("integration_github", "degraded", { lastError: "other" }),
    })],
  });
  await flushEvents();

  assert.equal(store.snapshot.sequence, 4);
  assert.deepEqual(store.snapshot.integrations, [
    integration("integration_github", "degraded", { lastError: "other" }),
  ]);
});

test("bridge failures remain explicit and a later sync can recover", async () => {
  const transport = new TestTransport();
  transport.responses.push(new BridgeCallError("broker_sync", "invocation_failed"));
  const store = new BrokerViewStore(transport);
  await store.start();

  assert.equal(store.status, "error");
  assert.equal(store.error?.name, "BridgeCallError");
  assert.match(store.error?.message ?? "", /broker_sync failed/);
  assert.equal(store.snapshot.sequence, 0);

  transport.responses.push({ kind: "snapshot", snapshot: emptySnapshot(2) });
  await store.synchronize(0);
  assert.equal(store.status, "ready");
  assert.equal(store.error, null);
  assert.equal(store.snapshot.sequence, 2);
});
