// App state — mirror of AppState.swift (the parts the island needs).

import type { BotEmoteName, BotStateName, IslandMode, IslandViewName } from "./layout";
import type { EyeShape } from "../mochi/engine";
import type { CapabilityRegistry } from "./capability-guard.js";

export type AgentSource = "claudeCode" | "n8n";
export type PillBadge = "approval" | "finished" | "error";

export interface TaskStep {
  /** Monotonic within this task, even after the bounded display history rolls over. */
  sequence: number;
  text: string;
}

export interface AgentTask {
  id: string;
  name: string;
  color: string;
  state: BotStateName;
  /** Local invalidation token for delayed UI work; never a Codex event ID. */
  activityGeneration: number;
  stepSequence: number;
  stepGeneration: number;
  steps: TaskStep[];
  source: AgentSource;
  isIntegration: boolean;
  emote?: BotEmoteName | null;
  miniEye?: EyeShape | null;
  pillBadge?: PillBadge | null;
  sessionCwd?: string | null;
}

export interface ApprovalInfo {
  requestId: string;
  sessionId: string;
  tool: string;
  command: string;
}

export interface ChatMessage {
  id: number;
  role: "user" | "assistant";
  content: string;
}

export type PromptContext =
  | { kind: "window"; appName: string; title: string; url?: string }
  | { kind: "file"; name: string; path?: string };

export interface ResultItem {
  label: string;
  detail: string;
  url?: string;
}

export interface SearchResult {
  title: string;
  items: ResultItem[];
  note?: string;
}

const task = (
  id: string, name: string, color: string, source: AgentSource,
): AgentTask => ({
  id, name, color, state: "idle", activityGeneration: 0,
  stepSequence: 0, stepGeneration: 0, steps: [], source, isIntegration: true,
});

/** AgentTask.integrationAgents — same ids, names and colours as macOS. */
export const INTEGRATION_AGENTS: AgentTask[] = [
  task("integration_claude", "VS Code", "#F5F6F8", "claudeCode"),
  task("integration_resend", "Resend", "#22C55E", "n8n"),
  task("integration_n8n", "n8n", "#F29B38", "n8n"),
  task("integration_vercel", "Vercel", "#7C5CFF", "n8n"),
  task("integration_github", "GitHub", "#F4505E", "n8n"),
  task("integration_notion", "Notion", "#8C8C8C", "n8n"),
  task("integration_calcom", "Cal.com", "#C9956A", "n8n"),
  task("integration_stripe", "Stripe", "#0570DE", "n8n"),
];

export const TOGGLEABLE_INTEGRATION_IDS = [
  "integration_resend", "integration_n8n", "integration_vercel", "integration_github",
  "integration_notion", "integration_calcom", "integration_stripe",
];

/** What an integration poller last reported. */
export interface IntegrationInfo {
  data: Record<string, unknown>;
  error: string | null;
  loaded: boolean;
  configured: boolean;
}

export interface Settings {
  soundEnabled: boolean;
  soundVolume: number;
  reducedMotion: boolean;
  autoCloseInterval: number;
  absenceInterval: number;
  activeIntegrations: string[];
  /** primary, cursor, or monitor:<Windows display name>. */
  screen: string;
  edgeOffset: number;
  toggleShortcut: string;
  autostart: boolean;
  hooksInstalled: boolean;
  /** Store redacted turn summaries and plans in bounded local history. */
  retainHistoryContent: boolean;
  /** Claude model used by the chat. */
  model: string;
}

export const DEFAULT_SETTINGS: Settings = {
  soundEnabled: true,
  soundVolume: 0.12,
  reducedMotion: false,
  autoCloseInterval: 15,
  absenceInterval: 180,
  activeIntegrations: [
    "integration_resend", "integration_n8n", "integration_vercel", "integration_github",
  ],
  screen: "primary",
  edgeOffset: 0,
  toggleShortcut: "CommandOrControl+Alt+Shift+Space",
  autostart: false,
  hooksInstalled: false,
  retainHistoryContent: false,
  model: "claude-opus-5",
};

type Listener = () => void;

class AppState {
  mode: IslandMode = "hidden";
  view: IslandViewName = "overview";

  tasks: AgentTask[] = [];
  focusId: string | null = null;

  stateOverride: BotStateName | null = null;

  /** Cursor in logical screen pixels, origin top-left (like AppState.mousePosition). */
  mouse = { x: 0, y: 0 };
  /** Cursor relative to the island's top-left corner. */
  mouseInIsland = { x: 0, y: 0 };

  isPinned = false;
  paused = false;

  uploadProgress = 0;
  uploadDuration = 2.4;
  fileDragOver = false;

  promptContext: PromptContext | null = null;
  droppedFile: { name: string; path: string } | null = null;
  noteMessage: string | null = null;
  searchResult: SearchResult | null = null;
  chatHistory: ChatMessage[] = [];
  pendingApproval: ApprovalInfo | null = null;

  integrations: Record<string, IntegrationInfo> = {};

  lastActivity = performance.now();

  settings: Settings = { ...DEFAULT_SETTINGS };
  /** Rust-owned runtime capabilities; absent in a plain-browser preview. */
  capabilities: CapabilityRegistry | null = null;

  private listeners = new Set<Listener>();
  /** Unique app-local tokens keep delayed work stale across task removal/re-addition. */
  private activityGeneration = 0;

  private nextActivityGeneration(): number {
    const floor = this.tasks.reduce((maximum, task) =>
      Number.isSafeInteger(task.activityGeneration)
        ? Math.max(maximum, task.activityGeneration)
        : maximum, this.activityGeneration);
    if (floor >= Number.MAX_SAFE_INTEGER) {
      throw new RangeError("Task activity generation exhausted its safe integer range.");
    }
    this.activityGeneration = floor + 1;
    return this.activityGeneration;
  }

  subscribe(fn: Listener): () => void {
    this.listeners.add(fn);
    return () => this.listeners.delete(fn);
  }

  /** Marks the UI dirty; the island re-renders on the next frame. */
  notify() {
    for (const fn of this.listeners) fn();
  }

  get focusTask(): AgentTask | null {
    return this.tasks.find((t) => t.id === this.focusId) ?? this.tasks[0] ?? null;
  }

  get effectiveState(): BotStateName {
    return this.stateOverride ?? this.focusTask?.state ?? "idle";
  }

  get otherTasks(): AgentTask[] {
    return this.tasks.filter((t) => t.id !== this.focusId);
  }

  setFocus(id: string) {
    const t = this.tasks.find((x) => x.id === id);
    if (!t) return;
    this.focusId = id;
    t.pillBadge = null;
    this.notify();
  }

  updateTask(id: string, state: BotStateName): number | null {
    const t = this.tasks.find((x) => x.id === id);
    if (!t) return null;
    t.activityGeneration = this.nextActivityGeneration();
    t.state = state;
    this.notify();
    return t.activityGeneration;
  }

  currentActivityGeneration(id: string): number | null {
    return this.tasks.find((x) => x.id === id)?.activityGeneration ?? null;
  }

  /** Apply delayed state only if no newer task activity invalidated its token. */
  transitionTaskIfGeneration(
    id: string,
    generation: number,
    expectedStates: readonly BotStateName[],
    nextState: BotStateName,
    cleanup: { clearSteps?: boolean; clearBadge?: boolean } = {},
  ): boolean {
    const t = this.tasks.find((x) => x.id === id);
    if (!t || t.activityGeneration !== generation || !expectedStates.includes(t.state)) return false;
    t.activityGeneration = this.nextActivityGeneration();
    t.state = nextState;
    if (cleanup.clearSteps) {
      t.stepGeneration += 1;
      t.steps = [];
    }
    if (cleanup.clearBadge) t.pillBadge = null;
    this.notify();
    return true;
  }

  appendStep(id: string, step: string) {
    const t = this.tasks.find((x) => x.id === id);
    if (!t) return;
    t.activityGeneration = this.nextActivityGeneration();
    t.stepSequence += 1;
    t.steps.push({ sequence: t.stepSequence, text: step });
    if (t.steps.length > 20) t.steps.shift();
    this.notify();
  }

  replaceSteps(id: string, steps: string[]) {
    const t = this.tasks.find((x) => x.id === id);
    if (!t) return;
    t.activityGeneration = this.nextActivityGeneration();
    t.stepGeneration += 1;
    t.steps = steps.map((text) => ({ sequence: ++t.stepSequence, text }));
    if (t.steps.length > 20) t.steps = t.steps.slice(-20);
  }

  clearSteps(id: string) {
    const t = this.tasks.find((x) => x.id === id);
    if (t) {
      t.activityGeneration = this.nextActivityGeneration();
      t.stepGeneration += 1;
      t.steps = [];
    }
  }

  setPillBadge(id: string, badge: PillBadge | null) {
    const t = this.tasks.find((x) => x.id === id);
    if (!t) return;
    t.pillBadge = badge;
    this.notify();
  }

  /** loadIntegrationTasks() — VS Code always on, the rest opt-in (max 4). */
  loadIntegrationTasks() {
    for (const proto of INTEGRATION_AGENTS) {
      const shouldLoad =
        proto.id === "integration_claude" || this.settings.activeIntegrations.includes(proto.id);
      const idx = this.tasks.findIndex((t) => t.id === proto.id);
      if (shouldLoad && idx < 0) {
        this.tasks.push({ ...proto, activityGeneration: this.nextActivityGeneration(), steps: [] });
      }
      if (!shouldLoad && idx >= 0) this.tasks.splice(idx, 1);
    }
    // Keep the declared order so pills never shuffle.
    const order = INTEGRATION_AGENTS.map((t) => t.id);
    this.tasks.sort((a, b) => order.indexOf(a.id) - order.indexOf(b.id));
    if (!this.focusId) this.focusId = "integration_claude";
    this.notify();
  }

  toggleIntegration(id: string) {
    if (id === "integration_claude") return;
    const active = this.settings.activeIntegrations;
    if (active.includes(id)) {
      this.settings.activeIntegrations = active.filter((x) => x !== id);
      if (this.focusId === id) this.focusId = "integration_claude";
    } else {
      if (active.length >= 4) return;
      this.settings.activeIntegrations = [...active, id];
    }
    this.loadIntegrationTasks();
  }

  defaultView(): IslandViewName {
    return this.tasks.length === 0 ? "empty" : "overview";
  }
}

export const State = new AppState();
