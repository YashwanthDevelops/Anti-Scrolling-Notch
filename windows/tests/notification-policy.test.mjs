import assert from "node:assert/strict";
import { test } from "node:test";
import { projectNotificationPresentation, safeNotificationTarget } from "../src/core/notification-policy.ts";

const base = {
  primary: "failure",
  badgeCount: 2,
  sound: "failure",
  triggers: [{ id: "f1", category: "failure", priority: "failure", title: "Failed", detail: "A turn failed", target: null, occurredAtUnixMs: 1 }],
  nativeToast: "disabled",
  suppressed: false,
};

test("notification presentation keeps policy priority, honors suppression and never sounds approval attention", () => {
  assert.deepEqual(projectNotificationPresentation(base, true), {
    badgeCount: 2,
    label: "Codex activity failed",
    priority: "failure",
    sound: "error",
    reveal: true,
  });
  assert.equal(projectNotificationPresentation(base, false).sound, null);
  const waiting = projectNotificationPresentation({ ...base, sound: "attention", primary: "waiting" }, true);
  assert.equal(waiting.sound, null);
  assert.equal(waiting.priority, "waiting");
  assert.deepEqual(projectNotificationPresentation({ ...base, suppressed: true }, true), {
    badgeCount: 0,
    label: "Codex activity failed",
    priority: "idle",
    sound: null,
    reveal: false,
  });
});

test("notification actions accept only generic Codex and credential-free https github.com links", () => {
  assert.deepEqual(safeNotificationTarget({ kind: "openCodex" }), { kind: "openCodex" });
  assert.deepEqual(safeNotificationTarget({ kind: "openGitHub", url: "https://github.com/org/repo/pull/7" }), {
    kind: "openGitHub",
    url: "https://github.com/org/repo/pull/7",
  });
  assert.equal(safeNotificationTarget({ kind: "openGitHub", url: "http://github.com/org/repo" }), null);
  assert.equal(safeNotificationTarget({ kind: "openGitHub", url: "https://github.com.evil.example/repo" }), null);
  assert.equal(safeNotificationTarget({ kind: "openGitHub", url: "https://user:secret@github.com/repo" }), null);
  assert.equal(safeNotificationTarget(null), null);
});
