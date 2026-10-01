import assert from "node:assert/strict";
import { after, test } from "node:test";
import { IslandStateMachine } from "../src/island/fsm.ts";

function fakeTimers() {
  const originalWindow = globalThis.window;
  let nextId = 1;
  let now = 0;
  const timers = new Map();
  globalThis.window = {
    setTimeout(callback, delay) {
      const id = nextId++;
      timers.set(id, { callback, due: now + delay });
      return id;
    },
    clearTimeout(id) {
      timers.delete(id);
    },
  };
  after(() => {
    globalThis.window = originalWindow;
  });
  return {
    advance(ms) {
      const end = now + ms;
      while (true) {
        const next = [...timers.entries()].sort((a, b) => a[1].due - b[1].due)[0];
        if (!next || next[1].due > end) break;
        timers.delete(next[0]);
        now = next[1].due;
        next[1].callback();
      }
      now = end;
    },
  };
}

test("pinned requests survive leave timers, Escape and explicit close calls", () => {
  const timers = fakeTimers();
  const fsm = new IslandStateMachine();
  fsm.forceHome();
  fsm.setPinned(true);
  fsm.mouseLeft();
  fsm.forcePetit();
  fsm.forceHidden();
  timers.advance(180_000);
  assert.equal(fsm.state, "home");
});

test("text interaction suspends and resumes the home auto-close timer", () => {
  const timers = fakeTimers();
  const fsm = new IslandStateMachine();
  fsm.forceHome();
  fsm.mouseLeft();
  timers.advance(5_000);
  fsm.setInteracting(true);
  timers.advance(15_000);
  assert.equal(fsm.state, "home");
  fsm.setInteracting(false);
  fsm.mouseLeft();
  timers.advance((fsm.homeToPetitDelay * 1000) - 1);
  assert.equal(fsm.state, "home");
  timers.advance(1);
  assert.equal(fsm.state, "petit");
});

test("a drag interaction keeps the compact island visible until its timer resumes", () => {
  const timers = fakeTimers();
  const fsm = new IslandStateMachine();
  fsm.forcePetit();
  fsm.mouseLeft();
  fsm.setInteracting(true);
  timers.advance((fsm.petitToHiddenDelay + 1) * 1000);
  assert.equal(fsm.state, "petit");
  fsm.setInteracting(false);
  fsm.mouseLeft();
  timers.advance(fsm.petitToHiddenDelay * 1000);
  assert.equal(fsm.state, "hidden");
});

test("the global shortcut cycles shell visibility and cannot hide a pinned request", () => {
  fakeTimers();
  const fsm = new IslandStateMachine();
  fsm.toggle();
  assert.equal(fsm.state, "petit");
  fsm.toggle();
  assert.equal(fsm.state, "home");
  fsm.toggle();
  assert.equal(fsm.state, "hidden");
  fsm.forceHome();
  fsm.setPinned(true);
  fsm.toggle();
  assert.equal(fsm.state, "home");
});
