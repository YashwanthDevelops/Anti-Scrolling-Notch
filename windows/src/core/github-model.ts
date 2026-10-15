import type { GithubHealth, GithubSnapshot } from "./bridge.js";

const HEALTH_LABELS: Record<GithubHealth, string> = {
  ready: "Connected",
  ghUnavailable: "GitHub CLI (gh) is unavailable",
  unauthenticated: "Sign in to gh before connecting",
  offline: "GitHub is offline",
  rateLimited: "GitHub rate limit reached",
  permissionDenied: "GitHub access was denied",
  error: "GitHub status is unavailable",
  notConnected: "Not connected",
};

export interface WorkflowHealthProjection {
  label: string;
  color: string;
  checksAreCurrent: boolean;
}

/** A green workflow status requires healthy broker state and a fresh ready snapshot. */
export function projectWorkflowHealth(
  connected: boolean,
  configuration: string | undefined,
  connection: string | undefined,
  snapshot: GithubSnapshot | null,
): WorkflowHealthProjection {
  const health = snapshot?.health ?? "notConnected";
  const checksAreCurrent = configuration === "configured"
    && connection === "healthy"
    && health === "ready"
    && snapshot?.stale === false;
  const label = !connected
    ? health === "notConnected" ? "Not connected" : HEALTH_LABELS[health] ?? "Not connected"
    : snapshot?.stale ? "Showing cached data"
      : checksAreCurrent ? "Connected"
        : HEALTH_LABELS[health] ?? "Connected · status limited";
  return {
    label,
    color: checksAreCurrent ? "#22C55E" : connected || snapshot ? "#F5A524" : "#A1A1AA",
    checksAreCurrent,
  };
}

/** Ignore scheduled snapshots from other repositories or after disconnect. */
export function acceptsWorkflowSnapshot(
  snapshot: GithubSnapshot,
  connected: boolean,
  selectedWorktreeId: string | null,
): boolean {
  return connected && selectedWorktreeId !== null && snapshot.repository.worktreeId === selectedWorktreeId;
}
