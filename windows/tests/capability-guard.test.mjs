import assert from "node:assert/strict";
import test from "node:test";

import { capabilityRequestGeneration } from "../src/core/capability-guard.js";

const verifiedOpenApp = {
  id: "codex.openApp",
  adapterAvailable: true,
  enabled: true,
};

function runtimeRegistry(overrides = {}) {
  return {
    schemaVersion: 1,
    generation: "cap-v1|family=OpenAI.Codex_2p2nqsd0c76g0|desktop=26.928.2636.0",
    runtimeSource: "windows-package-discovery",
    fixtureMode: false,
    installedCodexDesktopVersion: "26.928.2636.0",
    capabilities: [verifiedOpenApp],
    ...overrides,
  };
}

test("frontend can request only an enabled backend runtime capability", () => {
  const registry = runtimeRegistry();
  assert.equal(
    capabilityRequestGeneration(registry, "codex.openApp"),
    registry.generation,
  );
});

test("browser, demo, fixture and unknown registry snapshots fail closed", () => {
  for (const candidate of [
    null,
    runtimeRegistry({ fixtureMode: true }),
    runtimeRegistry({ runtimeSource: "demo-fixture" }),
    runtimeRegistry({ schemaVersion: 2 }),
    runtimeRegistry({ generation: "" }),
    runtimeRegistry({ capabilities: null }),
  ]) {
    assert.equal(capabilityRequestGeneration(candidate, "codex.openApp"), null);
  }
});

test("disabled, missing and adapterless capabilities cannot produce a request token", () => {
  for (const capability of [
    { ...verifiedOpenApp, enabled: false },
    { ...verifiedOpenApp, adapterAvailable: false },
  ]) {
    assert.equal(
      capabilityRequestGeneration(runtimeRegistry({ capabilities: [capability] }), "codex.openApp"),
      null,
    );
  }
  assert.equal(capabilityRequestGeneration(runtimeRegistry({ capabilities: [] }), "codex.openApp"), null);
  assert.equal(capabilityRequestGeneration(runtimeRegistry(), "codex.unlisted"), null);
});

test("fixture-supplied enabled flags cannot replace the backend-discovered capability list", () => {
  const forgedDemo = runtimeRegistry({
    runtimeSource: "demo-fixture",
    fixtureMode: true,
    capabilities: [{ ...verifiedOpenApp, enabled: true, adapterAvailable: true }],
  });
  assert.equal(capabilityRequestGeneration(forgedDemo, "codex.openApp"), null);
});
