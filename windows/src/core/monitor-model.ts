import type { BrokerRecord, BrokerSession, SourceScopedId } from "./broker-types.js";
import type { CodexHookStatus, CodexMonitorStatus } from "./bridge.js";

export interface SessionProjection {
  key: string;
  title: string;
  lifecycle: string;
  activity: string;
  waiting: string;
  connection: string;
  lastSeenAtUnixMs: number | null;
  startedAtUnixMs: number | null;
  turns: BrokerRecord[];
}

/** Stable source-scoped key; IDs from separate sources never collide. */
export function brokerIdentityKey(identity: SourceScopedId | null | undefined): string | null {
  if (!identity || typeof identity.source !== "string" || typeof identity.id !== "string") return null;
  return `${identity.source}:${identity.id}`;
}

/** Project only captured broker values. Display names and inferred labels stay out. */
export function projectSessions(snapshot: {
  readonly sessions: readonly BrokerSession[];
  readonly turns: readonly BrokerRecord[];
}): SessionProjection[] {
  return snapshot.sessions
    .filter((session) => session.identity?.source === "codex_cli_observer" && typeof session.identity.id === "string")
    .map((session) => {
      const key = brokerIdentityKey(session.identity)!;
      const turns = snapshot.turns.filter((turn) => {
        const linked = turn.sessionId as SourceScopedId | null | undefined;
        return brokerIdentityKey(linked) === key;
      });
      return {
        key,
        title: `Session ${shortOpaqueId(session.identity.id)}`,
        lifecycle: safeState(session.lifecycle),
        activity: safeState(session.activity),
        waiting: safeState(session.waiting),
        connection: safeState(session.connection),
        lastSeenAtUnixMs: safeTimestamp(session.lastSeenAtUnixMs),
        startedAtUnixMs: safeTimestamp(session.startedAtUnixMs),
        turns: turns.slice().sort((a, b) => timestamp(b.endedAtUnixMs ?? b.startedAtUnixMs) - timestamp(a.endedAtUnixMs ?? a.startedAtUnixMs)),
      };
    })
    .sort((a, b) => (b.lastSeenAtUnixMs ?? 0) - (a.lastSeenAtUnixMs ?? 0));
}

export function formatObservedTime(value: number | null): string {
  if (value == null || !Number.isFinite(value) || value <= 0) return "Not observed yet";
  try {
    return new Date(value).toLocaleString(undefined, { dateStyle: "medium", timeStyle: "short" });
  } catch {
    return "Not observed yet";
  }
}

export function displayState(value: string): string {
  if (!value || value === "unknown") return "Unknown";
  return value.replaceAll("_", " ");
}

export function shortOpaqueId(value: string): string {
  if (value.length <= 16) return value;
  return `${value.slice(0, 8)}…${value.slice(-4)}`;
}

export interface ReadinessProjection {
  observer: string;
  hookConfiguration: string;
  evidenceGate: string;
  trust: string;
  lastObserved: string;
  history: string;
  restartRecovery: string;
  productionEnabled: boolean;
}

/** Keep configured hook files separate from Codex's user-controlled trust decision. */
export function projectReadiness(
  monitor: CodexMonitorStatus | null,
  hooks: CodexHookStatus | null,
  monitorUnavailable = false,
  hooksUnavailable = false,
): ReadinessProjection {
  const hookConfiguration = hooksUnavailable
    ? "Hook configuration status unavailable"
    : !hooks
      ? "Configuration unknown"
      : hooks.configuredEvents.length > 0 && hooks.hookReady && hooks.cliSupported
        ? `Configured · ${hooks.configuredEvents.length} captured event types`
        : hooks.configuredEvents.length > 0
          ? "Configured · hook or CLI readiness needs attention"
          : "Not configured";
  return {
    observer: monitorUnavailable
      ? "Observer status unavailable"
      : monitor
        ? `${monitor.productionEnabled ? "Verified observer" : "Limited observer"} · ${displayState(monitor.observerState)}`
        : "Observer status unknown",
    hookConfiguration,
    evidenceGate: monitor
      ? `Hook registration ${monitor.hookRegistrationVerified ? "verified" : "unverified"} · relay ${monitor.relayReady ? "ready" : "not ready"} · cross-session identity ${monitor.crossSessionIdentityVerified ? "verified" : "unverified"}`
      : "Runtime evidence unavailable",
    trust: "Unknown · review with /hooks in Codex CLI",
    lastObserved: formatObservedTime(monitor?.lastEventAtUnixMs ?? null),
    history: monitor ? monitor.historyAvailable ? "Available" : "Waiting for first observation" : "Unknown",
    restartRecovery: monitor?.brokerStateRecovered === true
      ? "Retained session records restored · live status unknown"
      : monitor
        ? `${monitor.recoveryPartial ? "Partial history recovery" : "Live session state not restored"} · ${monitor.recoveredEventCount} retained observation${monitor.recoveredEventCount === 1 ? "" : "s"}`
        : "Unknown",
    productionEnabled: monitor?.productionEnabled === true,
  };
}

function safeState(value: unknown): string {
  return typeof value === "string" && value.length < 80 ? value : "unknown";
}

function safeTimestamp(value: unknown): number | null {
  return typeof value === "number" && Number.isFinite(value) && value > 0 ? value : null;
}

function timestamp(value: unknown): number {
  return safeTimestamp(value) ?? 0;
}
