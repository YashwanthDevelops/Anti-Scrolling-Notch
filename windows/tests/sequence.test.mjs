import assert from "node:assert/strict";
import { after, before, test } from "node:test";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { createServer } from "vite";

const originalDocument = globalThis.document;
const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
let vite;
let State;
let Ticker;
let expireIntegrationResult;
let expireStopResult;
let originalTasks;
let originalSettings;
let originalFocusId;

class TestElement {
  style = {};
  children = [];
  attributes = new Map();
  text = "";
  className = "";

  setAttribute(name, value) {
    this.attributes.set(name, value);
  }

  addEventListener() {}

  append(...children) {
    this.children.push(...children);
  }

  set textContent(value) {
    this.text = String(value);
    this.children = [];
  }

  get textContent() {
    return this.text || this.children.map((child) => child.textContent ?? "").join("");
  }
}

function addStep(task, text) {
  task.stepSequence += 1;
  task.steps.push({ sequence: task.stepSequence, text });
  if (task.steps.length > 20) task.steps.shift();
}

function finishAnimations(ticker) {
  let now = 0;
  for (let frame = 0; ticker.animating && frame < 20; frame++) {
    ticker.tick(now);
    now += 381;
  }
  assert.equal(ticker.animating, false, "all retained ticker steps should finish");
}

before(async () => {
  globalThis.document = {
    createElement: () => new TestElement(),
    createElementNS: () => new TestElement(),
    createTextNode: (text) => {
      const node = new TestElement();
      node.textContent = text;
      return node;
    },
  };
  vite = await createServer({
    root,
    configFile: false,
    server: { middlewareMode: true, watch: null, hmr: false },
    appType: "custom",
    optimizeDeps: { noDiscovery: true, include: [], holdUntilCrawlEnd: false },
  });
  ({ State } = await vite.ssrLoadModule("/src/core/state.ts"));
  ({ Ticker } = await vite.ssrLoadModule("/src/views/ticker.ts"));
  ({ expireIntegrationResult } = await vite.ssrLoadModule("/src/island/integrations.ts"));
  ({ expireStopResult } = await vite.ssrLoadModule("/src/island/hooks.ts"));
  originalTasks = State.tasks;
  originalSettings = { ...State.settings, activeIntegrations: [...State.settings.activeIntegrations] };
  originalFocusId = State.focusId;
});

after(async () => {
  if (State) {
    State.tasks = originalTasks;
    State.settings = originalSettings;
    State.focusId = originalFocusId;
  }
  if (originalDocument === undefined) delete globalThis.document;
  else globalThis.document = originalDocument;
  await vite?.close();
});

test("task step IDs stay monotonic while the bounded history rolls over and clears", () => {
  State.tasks = [{
    id: "sequence-test",
    activityGeneration: 0,
    stepSequence: 0,
    stepGeneration: 0,
    steps: [],
  }];

  for (let index = 1; index <= 24; index++) {
    State.appendStep("sequence-test", `step ${index}`);
  }

  const task = State.tasks[0];
  assert.equal(task.stepSequence, 24);
  assert.equal(task.steps.length, 20);
  assert.deepEqual(task.steps.map((step) => step.sequence), Array.from({ length: 20 }, (_, i) => i + 5));

  State.replaceSteps("sequence-test", ["replacement A", "replacement B"]);
  assert.deepEqual(task.steps.map((step) => step.sequence), [25, 26]);
  State.clearSteps("sequence-test");
  assert.equal(task.stepGeneration, 2);
  State.appendStep("sequence-test", "after clear");
  assert.deepEqual(task.steps, [{ sequence: 27, text: "after clear" }]);

  State.replaceSteps("sequence-test", Array.from({ length: 21 }, (_, index) => `replacement ${index}`));
  assert.equal(task.steps.length, 20);
  assert.deepEqual(task.steps.map((step) => step.sequence), Array.from({ length: 20 }, (_, i) => i + 29));
  assert.equal(task.stepSequence, 48);
});

test("a delayed completion transition cannot overwrite a newer activity generation", () => {
  State.tasks = [{
    id: "timer-test",
    state: "idle",
    activityGeneration: 0,
    stepSequence: 0,
    stepGeneration: 0,
    steps: [],
  }];

  const task = State.tasks[0];
  const earlierStop = State.updateTask("timer-test", "finished");
  const laterStop = State.updateTask("timer-test", "finished");
  assert.equal(
    State.transitionTaskIfGeneration("timer-test", earlierStop, ["finished"], "idle"),
    false,
    "an older stop cannot expire a later stop even when the visible state matches",
  );
  assert.equal(task.state, "finished");

  State.updateTask("timer-test", "thinking");
  assert.equal(
    State.transitionTaskIfGeneration("timer-test", laterStop, ["finished"], "idle"),
    false,
    "a prior stop cannot reset active work",
  );
  assert.equal(task.state, "thinking");

  State.updateTask("timer-test", "finished");
  State.appendStep("timer-test", "final output");
  const currentStop = State.currentActivityGeneration("timer-test");
  assert.equal(
    State.transitionTaskIfGeneration("timer-test", currentStop, ["finished"], "idle"),
    true,
    "the matching current stop can still expire normally",
  );
  assert.equal(task.state, "idle");

  const integrationError = State.updateTask("timer-test", "error");
  assert.equal(
    State.transitionTaskIfGeneration("timer-test", integrationError, ["finished", "error"], "idle"),
    true,
    "integration cleanup accepts either terminal result state",
  );
});

test("an old integration cleanup cannot match a removed and re-added task", () => {
  const id = "integration_github";
  State.tasks = [];
  State.settings.activeIntegrations = [id];
  State.loadIntegrationTasks();

  State.updateTask(id, "finished");
  State.replaceSteps(id, ["old result"]);
  const oldResult = State.currentActivityGeneration(id);

  State.settings.activeIntegrations = [];
  State.loadIntegrationTasks();
  assert.equal(State.currentActivityGeneration(id), null, "removing an integration removes its task");

  State.settings.activeIntegrations = [id];
  State.loadIntegrationTasks();
  const recreated = State.tasks.find((candidate) => candidate.id === id);
  assert.ok(recreated);
  State.updateTask(id, "finished");
  State.replaceSteps(id, ["new result"]);
  recreated.pillBadge = "finished";
  const newResult = State.currentActivityGeneration(id);

  assert.notEqual(oldResult, newResult, "each activity token is unique across task incarnations");
  assert.equal(expireIntegrationResult(id, oldResult), false, "the old timer cannot clean the recreated task");
  assert.equal(recreated.state, "finished");
  assert.deepEqual(recreated.steps.map((step) => step.text), ["new result"]);
  assert.equal(recreated.pillBadge, "finished");

  let observedExpiry;
  const unsubscribe = State.subscribe(() => {
    if (recreated.state === "idle") {
      observedExpiry = {
        steps: recreated.steps.map((step) => step.text),
        pillBadge: recreated.pillBadge,
      };
    }
  });
  assert.equal(expireIntegrationResult(id, newResult), true, "the current result still expires normally");
  unsubscribe();
  assert.deepEqual(observedExpiry, { steps: [], pillBadge: null }, "listeners observe the cleanup atomically");
  assert.equal(recreated.state, "idle");
  assert.deepEqual(recreated.steps, []);
  assert.equal(recreated.pillBadge, null);

  State.updateTask(id, "error");
  State.replaceSteps(id, ["error result"]);
  const errorGeneration = State.currentActivityGeneration(id);
  assert.equal(expireIntegrationResult(id, errorGeneration), true, "error results use the same cleanup path");
});

test("Stop cleanup preserves newer work and clears the matching finished badge", () => {
  State.tasks = [{
    id: "integration_claude",
    state: "finished",
    activityGeneration: 0,
    stepSequence: 0,
    stepGeneration: 0,
    steps: [],
    pillBadge: "finished",
  }];

  const staleStop = State.currentActivityGeneration("integration_claude");
  State.updateTask("integration_claude", "thinking");
  State.appendStep("integration_claude", "new prompt");
  assert.equal(expireStopResult(staleStop), false, "a prior Stop cannot reset newer activity");
  assert.equal(State.tasks[0].state, "thinking");
  assert.equal(State.tasks[0].steps[0].text, "new prompt");
  assert.equal(State.tasks[0].pillBadge, "finished");

  State.updateTask("integration_claude", "finished");
  const currentStop = State.currentActivityGeneration("integration_claude");
  assert.equal(expireStopResult(currentStop), true, "the current Stop result expires normally");
  assert.equal(State.tasks[0].state, "idle");
  assert.equal(State.tasks[0].pillBadge, null);
});

test("ticker sees new steps after history caps and keeps the latest burst rows", () => {
  const task = {
    id: "ticker-test",
    stepSequence: 20,
    stepGeneration: 0,
    steps: Array.from({ length: 20 }, (_, index) => ({
      sequence: index + 1,
      text: `step ${index + 1}`,
    })),
  };
  const ticker = new Ticker();
  ticker.sync(task);
  assert.equal(ticker.b.text, "step 20");

  for (let index = 21; index <= 24; index++) addStep(task, `step ${index}`);
  ticker.sync(task);
  assert.deepEqual(ticker.queue, ["step 21", "step 22", "step 23", "step 24"]);

  for (let index = 25; index <= 30; index++) addStep(task, `step ${index}`);
  ticker.sync(task);
  assert.deepEqual(ticker.queue, ["step 27", "step 28", "step 29", "step 30"]);
  finishAnimations(ticker);
  assert.equal(ticker.b.text, "step 30", "the latest content remains visible after a burst");

  task.steps = [];
  task.stepGeneration += 1;
  ticker.sync(task);
  assert.equal(ticker.queue.length, 0, "clearing a task cancels stale queued rows");
  assert.equal(ticker.b.text, "…");
  addStep(task, "new session step");
  ticker.sync(task);
  assert.deepEqual(ticker.queue, ["new session step"]);
  finishAnimations(ticker);
  assert.equal(ticker.b.text, "new session step");

  addStep(task, "queued before reset");
  ticker.sync(task);
  assert.deepEqual(ticker.queue, ["queued before reset"]);
  task.steps = [];
  task.stepGeneration += 1;
  addStep(task, "after coalesced reset");
  ticker.sync(task);
  assert.equal(ticker.queue.length, 0, "a clear and append before a render cannot replay stale rows");
  assert.equal(ticker.b.text, "after coalesced reset");

  const otherTask = {
    id: "other-task",
    stepSequence: 3,
    stepGeneration: 0,
    steps: [
      { sequence: 1, text: "other 1" },
      { sequence: 2, text: "other 2" },
      { sequence: 3, text: "other 3" },
    ],
  };
  ticker.sync(otherTask);
  assert.equal(ticker.queue.length, 0, "switching tasks seeds its current state without stale replay");
  assert.equal(ticker.b.text, "other 3");
});
