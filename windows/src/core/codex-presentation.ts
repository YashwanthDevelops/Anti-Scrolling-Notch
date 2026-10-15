import type { BrokerRecord, BrokerSession } from "./broker-types.js";
import { projectSessions, type SessionProjection } from "./monitor-model";
import type { BotStateName } from "./layout";

export interface CodexSessionPresentation {
  key: string;
  title: string;
  color: string;
  state: BotStateName;
  description: string;
}

/** Local presentation selection over backend-owned broker records. */
export class CodexPresentationStore {
  private sessionsValue: SessionProjection[] = [];
  private selectedKeyValue: string | null = null;
  private listeners = new Set<() => void>();

  get sessions(): readonly SessionProjection[] { return this.sessionsValue; }
  get selectedKey(): string | null { return this.selectedKeyValue; }
  get selected(): SessionProjection | null {
    return this.sessionsValue.find((session) => session.key === this.selectedKeyValue) ?? null;
  }

  /** Refresh only from broker snapshots; active-session selection stays local. */
  update(snapshot: { readonly sessions: readonly BrokerSession[]; readonly turns: readonly BrokerRecord[] }): void {
    const next = projectSessions(snapshot);
    const previousSignature = sessionSignature(this.sessionsValue);
    this.sessionsValue = next;
    if (!next.some((session) => session.key === this.selectedKeyValue)) {
      this.selectedKeyValue = next[0]?.key ?? null;
    }
    if (previousSignature !== sessionSignature(next)) this.notify();
  }

  select(key: string): void {
    if (!this.sessionsValue.some((session) => session.key === key) || key === this.selectedKeyValue) return;
    this.selectedKeyValue = key;
    this.notify();
  }

  subscribe(listener: () => void): () => void {
    this.listeners.add(listener);
    return () => this.listeners.delete(listener);
  }

  avatars(): CodexSessionPresentation[] {
    return this.sessionsValue.map(projectAvatar);
  }

  selectedAvatar(): CodexSessionPresentation | null {
    const selected = this.selected;
    return selected ? projectAvatar(selected) : null;
  }

  otherAvatars(limit = 4): CodexSessionPresentation[] {
    return this.sessionsValue
      .filter((session) => session.key !== this.selectedKeyValue)
      .slice(0, Math.max(0, limit))
      .map(projectAvatar);
  }

  get overflowCount(): number {
    return Math.max(0, this.sessionsValue.length - (this.selected ? 1 : 0) - 4);
  }

  private notify(): void {
    for (const listener of this.listeners) listener();
  }
}

export const CodexPresentation = new CodexPresentationStore();

export function projectSessionAvatar(session: SessionProjection): CodexSessionPresentation {
  return projectAvatar(session);
}

function projectAvatar(session: SessionProjection): CodexSessionPresentation {
  const state = sessionVisualState(session);
  return {
    key: session.key,
    title: session.title,
    color: "#A78BFA",
    state,
    description: sessionDescription(session, state),
  };
}

/** Unknown/offline and ended sessions stay neutral; stop is not a build result. */
export function sessionVisualState(session: SessionProjection): BotStateName {
  if (session.connection !== "healthy" || session.lifecycle !== "active") return "idle";
  if (session.waiting === "approval") return "approval";
  if (session.waiting === "user_input") return "question";
  if (session.waiting !== "not_waiting" && session.waiting !== "unknown") return "idle";
  if (session.turns.some((turn) => turn.lifecycle === "active")) return "thinking";
  if (session.activity === "working") return "thinking";
  return "idle";
}

function sessionDescription(session: SessionProjection, state: BotStateName): string {
  if (session.connection !== "healthy") return session.connection === "offline" ? "Observer offline · activity unknown" : "Observer connection unknown";
  if (session.lifecycle !== "active") return "Session ended · outcome unknown";
  if (session.waiting === "approval") return "Approval reported · respond in Codex";
  if (session.waiting === "user_input") return "Input requested · respond in Codex";
  if (state === "thinking") return "Activity observed · tool details unavailable";
  if (session.activity === "idle") return "Idle · no active turn observed";
  return "Activity unknown";
}

function sessionSignature(sessions: readonly SessionProjection[]): string {
  return sessions.map((session) => [
    session.key, session.lifecycle, session.activity, session.waiting, session.connection,
    session.lastSeenAtUnixMs,
    session.turns.map((turn) => `${turn.identity?.id ?? ""}:${turn.lifecycle}:${turn.endedAtUnixMs ?? ""}`).join(","),
  ].join("|")).join("~");
}
