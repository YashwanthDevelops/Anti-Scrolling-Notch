import assert from "node:assert/strict";
import { after, before, test } from "node:test";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { createServer } from "vite";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const windowDescriptor = Object.getOwnPropertyDescriptor(globalThis, "window");
let previewVite;
let nativeVite;
let PreviewBridge;
let Bridge;
let BridgeCallError;
let invocations;

function createViteServer() {
  return createServer({
    root,
    configFile: false,
    server: { middlewareMode: true, watch: null, hmr: false },
    appType: "custom",
    optimizeDeps: { noDiscovery: true, include: [], holdUntilCrawlEnd: false },
  });
}

before(async () => {
  delete globalThis.window;
  previewVite = await createViteServer();
  ({ Bridge: PreviewBridge } = await previewVite.ssrLoadModule("/src/core/bridge.ts"));

  invocations = [];
  Object.defineProperty(globalThis, "window", {
    configurable: true,
    value: {
      __TAURI_INTERNALS__: {
        invoke: async (command, args) => {
          invocations.push({ command, args });
          throw new Error("diagnostic-secret C:\\private\\settings.json");
        },
      },
    },
  });
  nativeVite = await createViteServer();
  ({ Bridge, BridgeCallError } = await nativeVite.ssrLoadModule("/src/core/bridge.ts"));
});

after(async () => {
  await Promise.all([previewVite?.close(), nativeVite?.close()]);
  if (windowDescriptor) Object.defineProperty(globalThis, "window", windowDescriptor);
  else delete globalThis.window;
});

function isBridgeFailure(command, kind) {
  return (error) => {
    assert.ok(error instanceof BridgeCallError);
    assert.equal(error.command, command);
    assert.equal(error.kind, kind);
    assert.equal(error.message.includes("diagnostic-secret"), false);
    assert.equal(error.message.includes("C:\\private"), false);
    assert.equal("original" in error, false);
    return true;
  };
}

test("plain-browser preview keeps its documented no-op result", async () => {
  assert.equal(await PreviewBridge.boot(), null);
});

test("plain-browser required commands fail with a typed runtime error", async () => {
  await assert.rejects(
    PreviewBridge.hooksPreview(false),
    (error) => error.name === "BridgeCallError"
      && error.command === "hooks_preview"
      && error.kind === "runtime_unavailable",
  );
});

test("native optional-result calls reject instead of converting failures to null", async () => {
  await assert.rejects(Bridge.boot(), isBridgeFailure("boot", "invocation_failed"));
  assert.deepEqual(invocations.filter(({ command }) => command === "boot"), [
    { command: "boot", args: {} },
  ]);
});

test("native required commands use the same typed, sanitized failure contract", async () => {
  await assert.rejects(
    Bridge.hooksPreview(false),
    isBridgeFailure("hooks_preview", "invocation_failed"),
  );
  assert.deepEqual(invocations.filter(({ command }) => command === "hooks_preview"), [
    { command: "hooks_preview", args: { install: false } },
  ]);
});

test("broker synchronization shares the native bridge error contract", async () => {
  await assert.rejects(
    Bridge.syncBroker({ afterSequence: 7 }),
    isBridgeFailure("broker_sync", "invocation_failed"),
  );
  assert.deepEqual(invocations.filter(({ command }) => command === "broker_sync"), [
    { command: "broker_sync", args: { afterSequence: 7 } },
  ]);
});
