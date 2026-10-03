// Integration events → island state. Port of the `handle…` methods in the Swift
// pollers: a genuinely new item flips the pill to finished/error, badges it when
// the pill isn't focused, plays a sound, and clears itself after 60 s.

import { onEvent, Bridge, reportBridgeFailure, type IntegrationUpdate } from "../core/bridge";
import { Sound } from "../core/sound";
import { State } from "../core/state";
import { BrokerView } from "../core/view-store";
import type { Island } from "./island";

/** Which Credential Manager key backs each pill. */
const KEY_FOR: Record<string, string> = {
  integration_stripe: "stripe-api-key",
  integration_github: "github-token",
  integration_vercel: "vercel-token",
  integration_n8n: "n8n-api-key",
  integration_resend: "resend-api-key",
  integration_notion: "notion-api-key",
  integration_calcom: "calcom-api-key",
};

const clearTimers = new Map<string, number>();

/** Apply a delayed integration-result cleanup only to its original result. */
export function expireIntegrationResult(id: string, generation: number): boolean {
  return State.transitionTaskIfGeneration(
    id, generation, ["finished", "error"], "idle", { clearSteps: true, clearBadge: true },
  );
}

export function registerIntegrationHandlers(island: Island, hooksStatusKnown = true) {
  BrokerView.subscribe(() => State.notify());
  void onEvent<IntegrationUpdate>("integration", (update) => handle(island, update));
  void refreshConfigured(hooksStatusKnown);
}

/** Asks Rust which keys exist so the idle cards can say so. */
export async function refreshConfigured(hooksStatusKnown = true) {
  for (const [id, key] of Object.entries(KEY_FOR)) {
    let present: boolean | null;
    try {
      present = await Bridge.secretPresent(key);
    } catch (error) {
      reportBridgeFailure(error, "integration credential status");
      const info = State.integrations[id];
      if (!info) State.integrations[id] = { data: {}, error: null, loaded: false, configured: null };
      continue;
    }
    const info = State.integrations[id] ?? { data: {}, error: null, loaded: false, configured: null };
    State.integrations[id] = present === null
      ? info
      : { ...info, configured: present };
  }
  const claude = State.integrations.integration_claude ?? {
    data: {}, error: null, loaded: false, configured: hooksStatusKnown ? State.settings.hooksInstalled : null,
  };
  State.integrations.integration_claude = hooksStatusKnown
    ? { ...claude, configured: State.settings.hooksInstalled }
    : claude;
  State.notify();
}

function handle(island: Island, update: IntegrationUpdate) {
  if (State.paused) return;

  const previous = State.integrations[update.id];
  State.integrations[update.id] = {
    data: update.error ? (previous?.data ?? {}) : update.data,
    error: update.error,
    loaded: update.error ? (previous?.loaded ?? false) : true,
    configured: previous?.configured ?? true,
  };

  const event = update.event;
  if (event) {
    const task = State.tasks.find((t) => t.id === update.id);
    if (task) {
      State.updateTask(update.id, event.success ? "finished" : "error");
      State.replaceSteps(update.id, event.detail ? [event.label, event.detail] : [event.label]);
      if (State.focusId !== update.id) {
        task.pillBadge = event.success ? "finished" : "error";
      }
      Sound.play(event.success ? "finish" : "error");
      // Same as the Swift pollers: show the compact island so the badge is seen,
      // but never steal the screen for a successful deploy.
      island.reveal();

      const existing = clearTimers.get(update.id);
      if (existing != null) window.clearTimeout(existing);
      const generation = State.currentActivityGeneration(update.id);
      if (generation == null) return;
      const timer = window.setTimeout(() => {
        if (clearTimers.get(update.id) === timer) clearTimers.delete(update.id);
        expireIntegrationResult(update.id, generation);
      }, 60_000);
      clearTimers.set(update.id, timer);
    }
  }

  State.notify();
}
