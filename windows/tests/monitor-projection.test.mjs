import assert from "node:assert/strict";
import { after, before, test } from "node:test";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { createServer } from "vite";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
let vite;
let projectSessions;
let formatObservedTime;
let projectReadiness;

before(async () => {
  vite = await createServer({
    root,
    configFile: false,
    server: { middlewareMode: true, watch: null, hmr: false },
    appType: "custom",
    optimizeDeps: { noDiscovery: true, include: [], holdUntilCrawlEnd: false },
  });
  ({ projectSessions, formatObservedTime, projectReadiness } = await vite.ssrLoadModule("/src/core/monitor-model.ts"));
});

after(async () => {
  await vite?.close();
});

const session = (id, lastSeenAtUnixMs, fields = {}) => ({
  schemaVersion: 1,
  identity: { source: "codex_cli_observer", id },
  lifecycle: "active",
  activity: "working",
  waiting: "unknown",
  connection: "healthy",
  startedAtUnixMs: null,
  lastSeenAtUnixMs,
  ...fields,
});

const turn = (id, sessionId, lifecycle, endedAtUnixMs) => ({
  schemaVersion: 1,
  identity: { source: "codex_cli_observer", id },
  sessionId: { source: "codex_cli_observer", id: sessionId },
  lifecycle,
  endedAtUnixMs,
});

test("projects independent sessions in observed order and binds turns by scoped identity", () => {
  const result = projectSessions({
    sessions: [
      session("older", 100, { model: null, repositoryId: null }),
      session("newer", 200, { lifecycle: "ended", activity: "idle" }),
    ],
    turns: [
      turn("turn-a", "older", "completed", 110),
      turn("turn-b", "newer", "active", 210),
      turn("unlinked", "elsewhere", "active", 220),
    ],
  });

  assert.deepEqual(result.map(({ key }) => key), [
    "codex_cli_observer:newer",
    "codex_cli_observer:older",
  ]);
  assert.deepEqual(result[0].turns.map((item) => item.identity.id), ["turn-b"]);
  assert.deepEqual(result[1].turns.map((item) => item.identity.id), ["turn-a"]);
  assert.equal(result[0].title, "Session newer");
  assert.equal(result[0].lifecycle, "ended");
  assert.equal("model" in result[0], false);
  assert.equal("project" in result[0], false);
});

test("keeps unknown captured lifecycle values and missing observation time explicit", () => {
  const [projection] = projectSessions({
    sessions: [session("unknown-state", null, { lifecycle: "unknown", activity: null })],
    turns: [],
  });

  assert.equal(projection.lifecycle, "unknown");
  assert.equal(projection.activity, "unknown");
  assert.equal(projection.lastSeenAtUnixMs, null);
  assert.equal(formatObservedTime(null), "Not observed yet");
});

test("readiness separates configured hooks, unknown trust, observer state, and last activity", () => {
  const monitor = {
    schemaVersion: 2,
    capability: "experimental_disabled",
    supportedCliVersion: "0.157.1",
    runtimeVersion: null,
    configuredHookEvents: ["SessionStart", "Stop"],
    hookRegistrationVerified: false,
    relayReady: true,
    crossSessionIdentityVerified: false,
    observerState: "listening",
    lastEventAtUnixMs: 5000,
    lastError: null,
    pipelineAvailable: true,
    productionEnabled: false,
    historyAvailable: true,
    brokerStateRecovered: false,
    recoveryPartial: true,
    recoveredEventCount: 3,
    capacityReached: false,
    disabledReason: "Runtime evidence is incomplete.",
  };
  const hooks = {
    configuredEvents: ["SessionStart", "Stop"],
    settingsPath: "hooks.json",
    hookPath: "observer.exe",
    hookReady: true,
    cliVersion: "0.157.1",
    cliSupported: true,
    supportedCliVersion: "0.157.1",
  };

  const projection = projectReadiness(monitor, hooks);
  assert.equal(projection.observer, "Limited observer · listening");
  assert.equal(projection.hookConfiguration, "Configured · 2 captured event types");
  assert.equal(projection.evidenceGate, "Hook registration unverified · relay ready · cross-session identity unverified");
  assert.match(projection.trust, /^Unknown/);
  assert.notEqual(projection.lastObserved, "Not observed yet");
  assert.equal(projection.history, "Available");
  assert.equal(projection.restartRecovery, "Partial history recovery · 3 retained observations");
  assert.equal(projection.productionEnabled, false);

  const recovered = projectReadiness({
    ...monitor,
    brokerStateRecovered: true,
    recoveryPartial: false,
  }, hooks);
  assert.equal(recovered.restartRecovery, "Retained session records restored · live status unknown");

  const failedRead = projectReadiness(null, null, true, true);
  assert.equal(failedRead.observer, "Observer status unavailable");
  assert.equal(failedRead.hookConfiguration, "Hook configuration status unavailable");
  assert.match(failedRead.trust, /^Unknown/);
});
