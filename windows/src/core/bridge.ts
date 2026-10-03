// Thin wrapper over the Tauri commands/events. Every call is a no-op when the
// page is opened in a plain browser, so the island can be iterated on with
// `npm run dev` alone.

import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { capabilityRequestGeneration } from "./capability-guard.js";
import type { BrokerSyncIntent, BrokerSyncResponse } from "./broker-types.js";
import { State, type Settings } from "./state";

export const IS_TAURI =
  typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

async function call<T>(cmd: string, args?: Record<string, unknown>): Promise<T | null> {
  if (!IS_TAURI) return null;
  try {
    return await invoke<T>(cmd, args);
  } catch (err) {
    console.error(`[anti-scrolling-notch] ${cmd} failed`, err);
    return null;
  }
}

export interface BootInfo {
  settings: Settings;
  /** Logical screen rect of the monitor the island lives on. */
  screen: { x: number; y: number; width: number; height: number; scale: number };
  monitors: MonitorOption[];
  shortcutStatus: ToggleShortcutStatus;
  version: string;
  hookPath: string;
  capabilities: import("./capability-guard.js").CapabilityRegistry;
}

export interface MonitorOption {
  id: string;
  label: string;
  scale: number;
}

export interface ToggleShortcutStatus {
  active: string | null;
  error: string | null;
}

export const Bridge = {
  boot: () => call<BootInfo>("boot"),

  /** Fetch an authoritative backend snapshot or contiguous replay. */
  syncBroker: (intent: BrokerSyncIntent) => syncBroker(intent),

  saveSettings: (settings: Settings) => call<void>("save_settings", { settings }),

  monitorOptions: () => call<MonitorOption[]>("monitor_options"),

  setToggleShortcut: (shortcut: string) =>
    callOrThrow<ToggleShortcutStatus>("set_toggle_shortcut", { shortcut }),

  setFileDragActive: (active: boolean) => call<void>("set_file_drag_active", { active }),

  /** Shrink the window down to the invisible wake strip (hidden) or back to full. */
  setCollapsed: (collapsed: boolean) => call<void>("set_collapsed", { collapsed }),

  /**
   * Pushes the island shape in window coordinates. Rust flips click-through from
   * its own cursor poll, so the flag is never a frame behind a click.
   */
  setIslandRect: (x: number, y: number, width: number, height: number) =>
    call<void>("set_island_rect", { x, y, width, height }),

  /** Give the window keyboard focus (chat field) and take it away again. */
  focusWindow: (focused: boolean) => call<void>("focus_window", { focused }),

  reposition: () => call<void>("reposition"),

  openUrl: (url: string) => call<void>("open_url", { url }),

  /** Best-effort activation of Codex's generic app surface, gated by Rust's current capability snapshot. */
  openCodex: () => {
    const expectedGeneration = capabilityRequestGeneration(State.capabilities, "codex.openApp");
    if (!expectedGeneration) {
      return Promise.reject(new Error("The verified Codex app activation capability is unavailable."));
    }
    return callOrThrow<number>("open_codex", { expectedGeneration });
  },

  /** "Open terminal" → opens the folder in VS Code when `code` is on PATH. */
  openInVSCode: (path: string | null) => call<boolean>("open_in_vscode", { path }),

  quit: () => call<void>("quit_app"),

  openSettingsWindow: () => call<void>("open_settings_window"),

  /** Writes to %LOCALAPPDATA%\Anti-Scrolling-Notch\anti-scrolling-notch.log. */
  log: (message: string) => call<void>("log_line", { message }),

  // ── Claude Code hooks ─────────────────────────────────────────────────────
  hooksStatus: () => call<HookStatus>("hooks_status"),
  /** Diff to show before anything is written. `install: false` previews removal. */
  hooksPreview: (install: boolean) => callOrThrow<HookPreview>("hooks_preview", { install }),
  /**
   * Writes ~/.claude/settings.json — only ever after an explicit click, and only
   * when the file still matches the preview the user looked at.
   */
  hooksApply: (install: boolean, fingerprint: string) =>
    callOrThrow<string>("hooks_apply", { install, fingerprint }),

  approvalDecision: (requestId: string, decision: "allow" | "deny") =>
    call<void>("approval_decision", { requestId, decision }),
  /** "The card is up" — until this lands the relay only waits a moment. */
  approvalAck: (requestId: string) => call<void>("approval_ack", { requestId }),
  /** "Nobody can act on this" — Claude Code asks in the terminal right away. */
  approvalDecline: (requestId: string) => call<void>("approval_decline", { requestId }),

  // ── Chat, files, secrets ──────────────────────────────────────────────────
  /** Managed chat stays unavailable until its backend capability is verified. */
  chatSend: (query: string, context: ChatContext | null) => {
    const generation = capabilityRequestGeneration(State.capabilities, "codex.managedSession");
    if (!generation) return Promise.reject(new Error("Companion chat is unavailable; continue in Codex."));
    if (context?.kind === "file" && !capabilityRequestGeneration(State.capabilities, "codex.attachmentDelivery")) {
      return Promise.reject(new Error("Codex file delivery is unavailable; attach the file in Codex."));
    }
    return callOrThrow<{ text: string }>("chat_send", { query, context, capabilityGeneration: generation });
  },
  /** Copies a dropped file only when the backend enables verified delivery. */
  ingestFile: (path: string) => {
    const generation = capabilityRequestGeneration(State.capabilities, "codex.attachmentDelivery");
    if (!generation) return Promise.reject(new Error("Codex file delivery is unavailable; attach the file in Codex."));
    return callOrThrow<DroppedFile>("ingest_file", { path, capabilityGeneration: generation });
  },
  /** Only ever tells you whether a key exists — never its value. */
  secretPresent: (key: string) => call<boolean>("secret_present", { key }),
  secretSet: (key: string, value: string) => callOrThrow<void>("secret_set", { key, value }),
  secretClear: (key: string) => callOrThrow<void>("secret_clear", { key }),

  // ── Integrations ──────────────────────────────────────────────────────────
  refreshIntegration: (id: string) => call<void>("refresh_integration", { id }),
  /** Opens the configured n8n instance in the browser. */
  openN8n: () => call<void>("open_n8n"),

  /** Tray → Pause. Stops the integration pollers, not just the island. */
  setPaused: (paused: boolean) => call<void>("set_paused", { paused }),
};

export interface IntegrationUpdate {
  id: string;
  data: Record<string, unknown>;
  error: string | null;
  event: { success: boolean; label: string; detail: string | null } | null;
}

export type ChatContext =
  | { kind: "file"; name: string; path: string }
  | { kind: "window"; appName: string; title: string; url?: string };

export interface DroppedFile {
  name: string;
  path: string;
  size: number;
}

export interface HookStatus {
  installed: boolean;
  settingsPath: string;
  hookPath: string;
  hookReady: boolean;
}

export interface HookPreview {
  diff: string;
  backup: string;
  settingsPath: string;
  /** Hand back to hooksApply so only the reviewed diff is ever written. */
  fingerprint: string;
}

/** Same as `call`, but surfaces the error so the UI can show what went wrong. */
async function callOrThrow<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (!IS_TAURI) throw new Error("not running inside Anti-Scrolling-Notch");
  return invoke<T>(cmd, args);
}

/** A typed failure for the new broker synchronization path only. */
async function syncBroker(intent: BrokerSyncIntent): Promise<BrokerSyncResponse> {
  const command = "broker_sync";
  try {
    if (!IS_TAURI) throw new Error("not running inside Anti-Scrolling-Notch");
    return await invoke<BrokerSyncResponse>(command, { afterSequence: intent.afterSequence });
  } catch (error) {
    throw new BridgeCallError(command, error);
  }
}

/** A rejected native invocation with its command name preserved for callers. */
export class BridgeCallError extends Error {
  readonly command: string;
  readonly original: unknown;

  constructor(command: string, original: unknown) {
    const detail = original instanceof Error ? original.message : String(original);
    super(`${command} failed: ${detail}`);
    this.name = "BridgeCallError";
    this.command = command;
    this.original = original;
  }
}

export type BridgeEvent =
  | { name: "broker-sync"; payload: BrokerSyncResponse }
  | { name: "cursor"; payload: { x: number; y: number } }
  | { name: "tray"; payload: string }
  | { name: "hook"; payload: Record<string, unknown> }
  | { name: "screen-changed"; payload: null }
  | { name: "toggle-island"; payload: null }
  | { name: "file-drag-cancelled"; payload: null };

export interface DragDropPayload {
  type: "enter" | "over" | "drop" | "leave";
  paths?: string[];
}

/** Files dragged onto the island. Only reaches us when the window takes the mouse. */
export async function onDragDrop(handler: (e: DragDropPayload) => void) {
  if (!IS_TAURI) return () => {};
  return getCurrentWebview().onDragDropEvent((event) => {
    handler(event.payload as DragDropPayload);
  });
}

export async function onEvent<T>(name: string, handler: (payload: T) => void) {
  if (!IS_TAURI) return () => {};
  return listen<T>(name, (e) => handler(e.payload));
}
