import assert from "node:assert/strict";
import { after, before, test } from "node:test";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { createServer } from "vite";

let vite;
let CodexPresentationStore;

before(async () => {
  const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
  vite = await createServer({
    root,
    configFile: false,
    server: { middlewareMode: true, watch: null, hmr: false },
    appType: "custom",
    optimizeDeps: { noDiscovery: true, include: [], holdUntilCrawlEnd: false },
  });
  ({ CodexPresentationStore } = await vite.ssrLoadModule("/src/core/codex-presentation.ts"));
});

after(async () => {
  await vite?.close();
});

const session = (id, lastSeenAtUnixMs, fields = {}) => ({
  schemaVersion: 1,
  identity: { source: "codex_cli_observer", id },
  lifecycle: "active",
  activity: "idle",
  waiting: "not_waiting",
  connection: "healthy",
  startedAtUnixMs: null,
  lastSeenAtUnixMs,
  ...fields,
});

const turn = (id, sessionId, lifecycle = "active") => ({
  schemaVersion: 1,
  identity: { source: "codex_cli_observer", id },
  sessionId: { source: "codex_cli_observer", id: sessionId },
  lifecycle,
  outcome: "unknown",
  startedAtUnixMs: 10,
  endedAtUnixMs: null,
});

const snapshot = (sessions, turns = []) => ({ sessions, turns });

test("selected session drives the main Mochi while another active session remains a mini avatar", () => {
  const store = new CodexPresentationStore();
  store.update(snapshot([
    session("quiet", 200),
    session("busy", 300, { activity: "working" }),
  ], [turn("busy-turn", "busy")]));

  const busy = store.sessions.find((item) => item.key.endsWith(":busy"));
  const quiet = store.sessions.find((item) => item.key.endsWith(":quiet"));
  assert.equal(store.selected?.key, busy.key); // most recently observed by default

  store.select(quiet.key);
  assert.equal(store.selectedAvatar().state, "idle");
  assert.match(store.selectedAvatar().description, /Idle/);
  const mini = store.otherAvatars(4);
  assert.equal(mini.length, 1);
  assert.equal(mini[0].key, busy.key);
  assert.equal(mini[0].state, "thinking");
  assert.match(mini[0].description, /tool details unavailable/);

  store.select(busy.key);
  assert.equal(store.selectedAvatar().state, "thinking");
});

test("attention is visual only; offline, unknown, ended, and restored states stay conservative", () => {
  const store = new CodexPresentationStore();
  store.update(snapshot([
    session("approval", 400, { waiting: "approval" }),
    session("offline", 300, { connection: "offline", activity: "working" }),
    session("unknown", 200, { connection: "unknown", activity: "working" }),
    session("ended", 100, { lifecycle: "ended" }),
  ], [turn("active-offline", "offline"), turn("active-unknown", "unknown"), turn("ended-turn", "ended", "completed")]));

  const byId = (id) => store.avatars().find((item) => item.key.endsWith(`:${id}`));
  assert.equal(store.selectedAvatar().state, "approval");
  assert.match(store.selectedAvatar().description, /respond in Codex/);
  assert.equal(byId("offline").state, "idle");
  assert.equal(byId("unknown").state, "idle");
  assert.equal(byId("ended").state, "idle");
  assert.match(byId("ended").description, /outcome unknown/);
});

test("selection stays local across broker updates and falls back when its session disappears", () => {
  const store = new CodexPresentationStore();
  store.update(snapshot([session("one", 200), session("two", 100)]));
  store.select("codex_cli_observer:two");
  store.update(snapshot([session("one", 300), session("two", 250, { activity: "working" })], [turn("t2", "two")]));
  assert.equal(store.selectedKey, "codex_cli_observer:two");
  assert.equal(store.selectedAvatar().state, "thinking");

  store.update(snapshot([session("one", 400)]));
  assert.equal(store.selectedKey, "codex_cli_observer:one");
  assert.equal(store.selectedAvatar().state, "idle");
});

test("session mini avatar projection caps the overview while selector data remains unbounded", () => {
  const store = new CodexPresentationStore();
  store.update(snapshot(Array.from({ length: 8 }, (_, index) => session(`s${index}`, 100 - index))));
  assert.equal(store.sessions.length, 8);
  assert.equal(store.otherAvatars(4).length, 4);
  assert.equal(store.overflowCount, 3);
});
