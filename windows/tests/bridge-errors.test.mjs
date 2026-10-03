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
let handleBridgeCall;
let safeBridgeFailure;
let refreshConfigured;
let State;
let invokeHandler;
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
          return invokeHandler(command, args);
        },
      },
    },
  });
  nativeVite = await createViteServer();
  ({ Bridge, BridgeCallError, handleBridgeCall, safeBridgeFailure } =
    await nativeVite.ssrLoadModule("/src/core/bridge.ts"));
  ({ refreshConfigured } = await nativeVite.ssrLoadModule("/src/island/integrations.ts"));
  ({ State } = await nativeVite.ssrLoadModule("/src/core/state.ts"));
  invokeHandler = async () => {
    throw new Error("diagnostic-secret C:\\private\\settings.json");
  };
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

test("caller recovery contains rejected actions and reports only safe failure metadata", async () => {
  const warnings = [];
  const originalWarn = console.warn;
  console.warn = (...args) => warnings.push(args.join(" "));
  let reported = 0;
  try {
    const result = await handleBridgeCall(
      Promise.reject(new BridgeCallError("approval_decision", "invocation_failed")),
      "approval action",
      () => { reported += 1; },
    );
    assert.equal(result, undefined);
    assert.equal(reported, 1);
    assert.equal(warnings.length, 1);
    assert.match(warnings[0], /approval_decision:invocation_failed/);
    assert.equal(warnings[0].includes("diagnostic-secret"), false);
    assert.equal(warnings[0].includes("C:\\private"), false);
  } finally {
    console.warn = originalWarn;
  }
});

test("caller recovery returns successful results without reporting a failure", async () => {
  const result = await handleBridgeCall(Promise.resolve("opened"), "open target", () => {
    assert.fail("success must not report a bridge failure");
  });
  assert.equal(result, "opened");
  assert.equal(safeBridgeFailure(new BridgeCallError("boot", "invocation_failed"), "fallback"), "boot failed");
  assert.equal(safeBridgeFailure(new Error("private payload"), "safe fallback"), "safe fallback");
});

test("failed credential checks retain prior configuration instead of reporting a missing key", async () => {
  State.integrations.integration_stripe = {
    data: {}, error: null, loaded: false, configured: true,
  };
  State.integrations.integration_github = {
    data: {}, error: null, loaded: false, configured: null,
  };
  const warnings = [];
  const originalWarn = console.warn;
  console.warn = (...args) => warnings.push(args.join(" "));
  invokeHandler = async (command, args) => {
    if (command !== "secret_present") throw new Error("unexpected command");
    if (args.key === "stripe-api-key") throw new Error("diagnostic-secret C:\\private\\settings.json");
    return args.key === "github-token" ? false : true;
  };
  try {
    await refreshConfigured(false);
    assert.equal(State.integrations.integration_stripe.configured, true);
    assert.equal(State.integrations.integration_github.configured, false);
    assert.equal(State.integrations.integration_vercel.configured, true);
    assert.equal(warnings.length, 1);
    assert.equal(warnings[0].includes("diagnostic-secret"), false);
    assert.equal(warnings[0].includes("C:\\private"), false);
  } finally {
    console.warn = originalWarn;
  }
});
