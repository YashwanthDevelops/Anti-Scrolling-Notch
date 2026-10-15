import assert from "node:assert/strict";
import { test } from "node:test";
import { escapeBlursIslandTextEntry, isIslandTextEntryFocused } from "../src/core/text-entry-focus.ts";
import { IslandStateMachine } from "../src/island/fsm.ts";

function fakeTimers() {
  const originalWindow = globalThis.window;
  let now = 0;
  let nextId = 1;
  const timers = new Map();
  globalThis.window = {
    setTimeout(callback, delay) {
      const id = nextId++;
      timers.set(id, { callback, due: now + delay });
      return id;
    },
    clearTimeout(id) { timers.delete(id); },
  };
  return {
    restore() { globalThis.window = originalWindow; },
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

test("repository path focus activates the island and suspends collapse until blur", () => {
  const timers = fakeTimers();
  try {
    const fsm = new IslandStateMachine();
    fsm.forceHome();
    fsm.mouseLeft();

    let nativeFocus = false;
    let interacting = false;
    const sync = (view, entry, documentFocused) => {
      const focused = isIslandTextEntryFocused(view, entry, documentFocused);
      if (focused !== nativeFocus) nativeFocus = focused;
      if (focused !== interacting) {
        interacting = focused;
        fsm.setInteracting(interacting);
        if (!interacting) fsm.mouseLeft();
      }
    };

    timers.advance(10_000);
    sync("history", "github-path", true);
    timers.advance(60_000);
    assert.equal(nativeFocus, true);
    assert.equal(fsm.state, "home");

    sync("history", null, true); // blur
    timers.advance(fsm.homeToPetitDelay * 1000 - 1);
    assert.equal(fsm.state, "home");
    timers.advance(1);
    assert.equal(fsm.state, "petit");
  } finally {
    timers.restore();
  }
});

test("Escape blurs only the active history repository path", () => {
  assert.equal(escapeBlursIslandTextEntry("Escape", "history", "github-path"), true);
  assert.equal(escapeBlursIslandTextEntry("Escape", "prompt", "chat"), false);
  assert.equal(escapeBlursIslandTextEntry("Escape", "history", null), false);
  assert.equal(escapeBlursIslandTextEntry("Enter", "history", "github-path"), false);
  assert.equal(isIslandTextEntryFocused("history", "github-path", false), false);
  assert.equal(isIslandTextEntryFocused("overview", "github-path", true), false);
});
