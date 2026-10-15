export type NotificationPriority = "waiting" | "failure" | "active" | "completion" | "idle";
export type NotificationSound = "attention" | "failure" | "activity" | "completion" | null;

export interface NotificationUpdate {
  primary: NotificationPriority;
  badgeCount: number;
  sound: NotificationSound;
  triggers: Array<{
    id: string;
    category: "waiting" | "failure" | "active" | "completion" | "github" | "health";
    priority: NotificationPriority;
    title: string;
    detail: string;
    target: { kind: "openCodex" } | { kind: "openGitHub"; url: string } | null;
    occurredAtUnixMs: number;
  }>;
  nativeToast: "disabled" | "unavailable" | "submitted_unverified";
  suppressed: boolean;
}

export interface NotificationPresentation {
  badgeCount: number;
  label: string;
  priority: NotificationPriority;
  sound: "error" | "work" | "finish" | null;
  reveal: boolean;
}

/** Pure projection of backend policy into the existing island badge/sound affordances. */
export function projectNotificationPresentation(
  update: NotificationUpdate,
  soundEnabled: boolean,
): NotificationPresentation {
  const priority = update.primary;
  const priorityLabels: Record<NotificationPriority, string> = {
    waiting: "Codex needs attention",
    failure: "Codex activity failed",
    active: "Codex activity updated",
    completion: "Codex turn completed",
    idle: "Codex activity",
  };
  const count = update.suppressed
    ? 0
    : Math.max(0, Math.trunc(update.badgeCount), update.triggers.length);
  const sound = !update.suppressed && soundEnabled
    ? update.sound === "failure" ? "error"
      : update.sound === "activity" ? "work"
        : update.sound === "completion" ? "finish"
          : null // Attention may represent an unsupported approval/input request.
    : null;
  return {
    badgeCount: count,
    label: priorityLabels[priority] ?? priorityLabels.idle,
    priority: update.suppressed ? "idle" : priority,
    sound,
    reveal: count > 0 && (priority === "waiting" || priority === "failure" || priority === "active" || priority === "completion"),
  };
}

/** Targets accepted by the notification UI remain deliberately narrow. */
export function safeNotificationTarget(
  target: NotificationUpdate["triggers"][number]["target"],
): { kind: "openCodex" } | { kind: "openGitHub"; url: string } | null {
  if (target?.kind === "openCodex") return { kind: "openCodex" };
  if (target?.kind !== "openGitHub") return null;
  try {
    const url = new URL(target.url);
    if (url.protocol !== "https:" || url.hostname.toLowerCase() !== "github.com" || url.username || url.password) return null;
    return { kind: "openGitHub", url: url.toString() };
  } catch {
    return null;
  }
}
