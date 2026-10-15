import assert from "node:assert/strict";
import { after, before, test } from "node:test";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { createServer } from "vite";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
let vite;
let acceptsWorkflowSnapshot;
let projectWorkflowHealth;

before(async () => {
  vite = await createServer({
    root,
    configFile: false,
    server: { middlewareMode: true, watch: null, hmr: false },
    appType: "custom",
    optimizeDeps: { noDiscovery: true, include: [], holdUntilCrawlEnd: false },
  });
  ({ acceptsWorkflowSnapshot, projectWorkflowHealth } = await vite.ssrLoadModule("/src/core/github-model.ts"));
});

after(async () => {
  await vite?.close();
});

function snapshot(overrides = {}) {
  return {
    repository: {
      canonicalPath: "C:/repo",
      commonDir: "C:/repo/.git",
      worktreeDir: "C:/repo",
      branch: "feature/work",
      headSha: "abcdef0123456789",
      remoteName: "origin",
      remoteUrl: "https://github.com/example/repo.git",
      owner: "example",
      name: "repo",
      repositoryId: "repo-1",
      worktreeId: "worktree-1",
    },
    pullRequest: null,
    checks: [],
    health: "ready",
    fetchedAtUnixMs: 100,
    stale: false,
    error: null,
    ...overrides,
  };
}

test("green workflow health requires configured healthy broker state and a fresh ready snapshot", () => {
  const current = projectWorkflowHealth(true, "configured", "healthy", snapshot());
  assert.equal(current.label, "Connected");
  assert.equal(current.color, "#22C55E");
  assert.equal(current.checksAreCurrent, true);

  const cached = projectWorkflowHealth(true, "configured", "healthy", snapshot({ stale: true }));
  assert.equal(cached.label, "Showing cached data");
  assert.equal(cached.color, "#F5A524");
  assert.equal(cached.checksAreCurrent, false);

  const brokerUnavailable = projectWorkflowHealth(true, "configured", "offline", snapshot());
  assert.equal(brokerUnavailable.color, "#F5A524");
  assert.equal(brokerUnavailable.checksAreCurrent, false);
});

test("scheduled updates are ignored after disconnect or when they belong to another worktree", () => {
  const update = snapshot();
  assert.equal(acceptsWorkflowSnapshot(update, true, "worktree-1"), true);
  assert.equal(acceptsWorkflowSnapshot(update, false, "worktree-1"), false);
  assert.equal(acceptsWorkflowSnapshot(update, true, "worktree-2"), false);
  assert.equal(acceptsWorkflowSnapshot(update, true, null), false);
});
