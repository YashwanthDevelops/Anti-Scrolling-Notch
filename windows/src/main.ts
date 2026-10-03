// Entry point: boot the bridge, wire the island, start the greeting.

import "./style.css";
import { Bridge, handleBridgeCall, IS_TAURI, onEvent, reportBridgeFailure } from "./core/bridge";
import { BrokerView } from "./core/view-store";
import { Sound } from "./core/sound";
import { State, type Settings } from "./core/state";
import { Island } from "./island/island";
import { registerHookHandlers } from "./island/hooks";
import { registerIntegrationHandlers, refreshConfigured } from "./island/integrations";

async function main() {
  const root = document.getElementById("root");
  if (!root) return;

  void Sound.preload();

  const island = new Island(root);

  let boot: Awaited<ReturnType<typeof Bridge.boot>> = null;
  let bootFailed = false;
  try {
    boot = await Bridge.boot();
  } catch (error) {
    bootFailed = true;
    reportBridgeFailure(error, "application startup settings");
  }
  if (boot) {
    State.settings = { ...State.settings, ...boot.settings };
    State.capabilities = boot.capabilities;
  }
  if (IS_TAURI) await BrokerView.start();
  island.applySettings();
  State.loadIntegrationTasks();

  await onEvent<{ x: number; y: number }>("cursor", ({ x, y }) => island.onCursor(x, y));

  /** Pause has to reach Rust too, or the pollers keep calling out. */
  let confirmedPaused = State.paused;
  let pauseIntent = 0;
  let pauseQueue = Promise.resolve();
  const setPaused = (on: boolean) => {
    if (State.paused === on) return;
    State.paused = on;
    const intent = ++pauseIntent;
    pauseQueue = pauseQueue.then(async () => {
      try {
        await Bridge.setPaused(on);
        confirmedPaused = on;
      } catch (error) {
        reportBridgeFailure(error, "tray pause control");
        if (intent === pauseIntent) {
          State.paused = confirmedPaused;
          State.notify();
          island.showBridgeFailure("Could not change the integration pause state.");
        }
      }
    });
  };

  await onEvent<string>("tray", (what) => {
    switch (what) {
      case "settings":
        setPaused(false);
        island.alert("settings");
        break;
      case "open":
        setPaused(false);
        island.alert(State.defaultView());
        break;
      case "pause":
        setPaused(!State.paused);
        if (State.paused) island.fsm.forceHidden();
        else island.reveal();
        break;
    }
  });

  await onEvent<null>("toggle-island", () => {
    Sound.resume();
    if (State.pendingApproval) {
      island.alert("approval");
      return;
    }
    island.fsm.toggle();
  });

  await onEvent<null>("screen-changed", () => {
    void handleBridgeCall(Bridge.reposition(), "screen reposition");
  });

  // The settings window writes preferences; apply them here without a restart.
  await onEvent<Settings>("settings-changed", (s) => {
    State.settings = { ...State.settings, ...s };
    island.applySettings();
    State.loadIntegrationTasks();
    void refreshConfigured();
  });

  registerHookHandlers(island);
  registerIntegrationHandlers(island, boot !== null);

  island.launch();
  if (bootFailed) {
    island.showBridgeFailure("Could not load Anti-Scrolling-Notch settings. Some status may be unavailable.");
  }

  // In a plain browser there is no wake strip behind the cursor: make the whole
  // page wake the island so the visuals can be checked with `npm run dev`.
  if (!IS_TAURI) {
    document.addEventListener("click", () => Sound.resume(), { once: true });
  }
}

void main();
