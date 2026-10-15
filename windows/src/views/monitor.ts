import { Bridge, safeBridgeFailure, type BrokerHistoryRecord, type BrokerHistoryPage, type CodexMonitorStatus } from "../core/bridge";
import { BrokerView } from "../core/view-store";
import { displayState, formatObservedTime, projectReadiness, projectSessions, shortOpaqueId, type SessionProjection } from "../core/monitor-model";
import { h, clear, dot } from "./dom";
import { safeNotificationTarget, type NotificationUpdate } from "../core/notification-policy";
import { CodexPresentation } from "../core/codex-presentation";

export interface MonitorViewActions {
  openSettingsWindow(): void;
  openCodex(): void;
  runBridgeAction<T>(operation: Promise<T>, failureMessage: string): void;
}

const HISTORY_PAGE_SIZE = 8;

/** Focused observer status, captured session detail, and persisted event metadata. */
export function buildMonitorView(actions: MonitorViewActions) {
  const statusLine = h("div", { class: "monitor-status" });
  const setupLine = h("div", { class: "monitor-setup" });
  const notificationList = h("div", { class: "monitor-notifications", "aria-live": "polite" });
  const sessionList = h("div", { class: "monitor-session-list", "aria-label": "Observed Codex sessions" });
  const detail = h("div", { class: "monitor-detail" });
  const historyList = h("div", { class: "monitor-history-list" });
  const refreshHistory = h("button", { class: "link-btn monitor-more", text: "Refresh history" });
  const moreHistory = h("button", { class: "link-btn monitor-more", text: "Load more history" });
  const refreshButton = h("button", { class: "link-btn monitor-refresh", text: "Refresh status" });
  const el = h("div", { class: "view monitor-view" }, h("div", { class: "card monitor-card" },
    h("div", { class: "monitor-head" },
      h("div", { class: "monitor-title" }, h("b", { text: "Codex activity" }), statusLine),
      refreshButton,
    ),
    setupLine,
    notificationList,
    h("div", { class: "monitor-columns" },
      h("section", { class: "monitor-sessions" },
        h("div", { class: "monitor-section-title", text: "Sessions" }), sessionList,
      ),
      h("section", { class: "monitor-inspector" }, detail),
    ),
    h("div", { class: "monitor-history" },
      h("div", { class: "monitor-section-title", text: "Saved observations" }),
      refreshHistory,
      historyList,
      moreHistory,
    ),
  ));

  let monitorStatus: CodexMonitorStatus | null = null;
  let monitorStatusError: string | null = null;
  let hookStatus: Awaited<ReturnType<typeof Bridge.codexHooksStatus>> | null = null;
  let hookStatusError: string | null = null;
  let statusLoading = false;
  let lastProjectionKey = "";
  let history: BrokerHistoryRecord[] = [];
  let historyNextId: number | null = null;
  let historyHasMore = false;
  let historyLoading = false;
  let historyLoaded = false;
  let historyError: string | null = null;
  let notificationUpdate: NotificationUpdate | null = null;

  const refreshStatus = async () => {
    if (statusLoading) return;
    statusLoading = true;
    monitorStatusError = null;
    hookStatusError = null;
    renderStatus();
    const [monitorResult, hookResult] = await Promise.allSettled([
      Bridge.codexMonitorStatus(),
      Bridge.codexHooksStatus(),
    ]);
    monitorStatus = monitorResult.status === "fulfilled" ? monitorResult.value : null;
    hookStatus = hookResult.status === "fulfilled" ? hookResult.value : null;
    monitorStatusError = monitorResult.status === "rejected"
      ? safeBridgeFailure(monitorResult.reason, "Observer status is unavailable.")
      : null;
    hookStatusError = hookResult.status === "rejected"
      ? safeBridgeFailure(hookResult.reason, "Hook configuration status is unavailable.")
      : null;
    statusLoading = false;
    renderStatus();
  };

  const loadHistory = async (append: boolean, force = false) => {
    if (historyLoading || (!append && historyLoaded && !force)) return;
    historyLoading = true;
    historyError = null;
    if (!append) {
      history = [];
      historyNextId = null;
      historyHasMore = false;
    }
    renderHistory();
    try {
      const page: BrokerHistoryPage = await Bridge.brokerHistory(append ? historyNextId : null, HISTORY_PAGE_SIZE);
      history = append ? [...history, ...page.records] : page.records;
      historyNextId = page.nextAfterId;
      historyHasMore = page.hasMore;
      historyLoaded = true;
    } catch (error) {
      historyError = safeBridgeFailure(error, "Saved observation history is unavailable.");
    } finally {
      historyLoading = false;
      renderHistory();
    }
  };

  refreshButton.addEventListener("click", () => void refreshStatus());
  refreshHistory.addEventListener("click", () => void loadHistory(false, true));
  moreHistory.addEventListener("click", () => void loadHistory(true));
  void refreshStatus();
  void loadHistory(false);

  function renderStatus() {
    const readiness = projectReadiness(monitorStatus, hookStatus, !!monitorStatusError, !!hookStatusError);
    clear(statusLine);
    if (statusLoading) {
      statusLine.append(dot("#A1A1AA", 6), h("span", { text: "Checking observer…" }));
    } else if (monitorStatus) {
      const color = monitorStatus.productionEnabled && monitorStatus.observerState === "listening" ? "#22C55E"
        : monitorStatus.observerState === "degraded" || monitorStatus.observerState === "listening" ? "#F5A524"
          : "#A1A1AA";
      const title = monitorStatus.productionEnabled ? "Verified observer" : "Limited observer";
      statusLine.append(dot(color, 6), h("span", { text: `${title} · ${displayState(monitorStatus.observerState)}` }));
    } else {
      statusLine.append(dot(monitorStatusError ? "#F5A524" : "#A1A1AA", 6), h("span", { text: readiness.observer }));
    }
    clear(setupLine);
    if (monitorStatus?.disabledReason && !monitorStatus.productionEnabled) {
      setupLine.append(h("p", { class: "monitor-limitation", text: monitorStatus.disabledReason }));
    }
    if (monitorStatus?.lastError) {
      setupLine.append(h("p", { class: "monitor-limitation", text: `Observer detail: ${monitorStatus.lastError}` }));
    }
    if (monitorStatus?.capacityReached) {
      setupLine.append(h("p", { class: "monitor-limitation", text: "Local observation history reached its storage limit." }));
    }
    setupLine.append(
      h("div", { class: "monitor-readiness" },
        h("span", { class: "monitor-readiness-label", text: "Hook configuration" }),
        h("span", { text: readiness.hookConfiguration }),
      ),
      h("div", { class: "monitor-readiness" },
        h("span", { class: "monitor-readiness-label", text: "Runtime evidence" }),
        h("span", { text: readiness.evidenceGate }),
      ),
      h("div", { class: "monitor-readiness" },
        h("span", { class: "monitor-readiness-label", text: "Codex trust" }),
        h("span", { text: readiness.trust }),
      ),
      h("div", { class: "monitor-readiness" },
        h("span", { class: "monitor-readiness-label", text: "Last observed" }),
        h("span", { text: readiness.lastObserved }),
      ),
      h("div", { class: "monitor-readiness" },
        h("span", { class: "monitor-readiness-label", text: "Saved history" }),
        h("span", { text: readiness.history }),
      ),
      h("div", { class: "monitor-readiness" },
        h("span", { class: "monitor-readiness-label", text: "Restart recovery" }),
        h("span", { text: readiness.restartRecovery }),
      ),
      h("div", { class: "monitor-actions" },
        h("button", { class: "link-btn", text: "Hook settings…", onclick: () => actions.openSettingsWindow() }),
        h("button", { class: "link-btn", text: "Open Codex", onclick: () => actions.openCodex() }),
      ),
    );
  }

  function renderSessions(sessions: SessionProjection[]) {
    if (sessions.length && !sessions.some((session) => session.key === CodexPresentation.selectedKey)) {
      CodexPresentation.select(sessions[0].key);
    }
    const selectedKey = CodexPresentation.selectedKey;
    const nextKey = sessions.map((session) => [
      session.key, session.lifecycle, session.activity, session.waiting, session.connection,
      session.lastSeenAtUnixMs, session.turns.map((turn) => JSON.stringify(turn)).join(";"),
    ].join("|")).join("~") + `:${selectedKey ?? "none"}`;
    if (nextKey === lastProjectionKey) return;
    lastProjectionKey = nextKey;
    clear(sessionList);
    if (!sessions.length) {
      sessionList.append(h("div", { class: "monitor-empty", text: "No Codex sessions have been observed yet." }));
      renderDetail(null);
      return;
    }
    for (const session of sessions) {
      const button = h("button", {
        class: `monitor-session${session.key === selectedKey ? " selected" : ""}`,
        "aria-pressed": session.key === selectedKey ? "true" : "false",
        onclick: () => {
          CodexPresentation.select(session.key);
        },
      },
        h("b", { text: session.title }),
        h("span", { text: `${displayState(session.lifecycle)} · ${displayState(session.activity)}` }),
        h("small", { text: `Last observed ${formatObservedTime(session.lastSeenAtUnixMs)}` }),
      );
      sessionList.append(button);
    }
    renderDetail(sessions.find((session) => session.key === selectedKey) ?? null);
  }

  function renderDetail(session: SessionProjection | null) {
    clear(detail);
    if (!session) {
      detail.append(h("div", { class: "monitor-empty", text: "Select a session to view its observed state." }));
      return;
    }
    const facts = h("div", { class: "monitor-facts" });
    for (const [label, value] of [
      ["Lifecycle", displayState(session.lifecycle)],
      ["Activity", displayState(session.activity)],
      ["Waiting", displayState(session.waiting)],
      ["Connection", displayState(session.connection)],
      ["Started", formatObservedTime(session.startedAtUnixMs)],
      ["Last observed", formatObservedTime(session.lastSeenAtUnixMs)],
    ]) {
      facts.append(h("div", { class: "monitor-fact" }, h("span", { text: label }), h("b", { text: value })));
    }
    const turns = h("div", { class: "monitor-turns" }, h("div", { class: "monitor-section-title", text: "Observed turns" }));
    if (!session.turns.length) turns.append(h("div", { class: "monitor-empty", text: "No turn details have been observed." }));
    for (const turn of session.turns.slice(0, 5)) {
      const id = turn.identity?.id ?? "";
      const lifecycle = typeof turn.lifecycle === "string" ? displayState(turn.lifecycle) : "Unknown";
      const observed = formatObservedTime(asTimestamp(turn.endedAtUnixMs ?? turn.startedAtUnixMs));
      turns.append(h("div", { class: "monitor-turn" },
        h("b", { text: id ? `Turn ${shortOpaqueId(id)}` : "Observed turn" }),
        h("span", { text: `${lifecycle} · ${observed}` }),
      ));
    }
    detail.append(h("div", { class: "monitor-detail-title", text: session.title }), facts, turns);
  }

  function renderHistory() {
    clear(historyList);
    if (historyError) historyList.append(h("div", { class: "monitor-empty", text: historyError }));
    else if (!history.length && historyLoading) historyList.append(h("div", { class: "monitor-empty", text: "Loading saved observations…" }));
    else if (!history.length) historyList.append(h("div", { class: "monitor-empty", text: "No saved observations yet." }));
    for (const record of history.slice().reverse()) {
      historyList.append(historyRow(record));
    }
    moreHistory.style.display = historyHasMore ? "" : "none";
    refreshHistory.toggleAttribute("disabled", historyLoading);
    refreshHistory.textContent = historyLoading ? "Refreshing…" : "Refresh history";
    moreHistory.textContent = historyLoading ? "Loading…" : "Load more history";
    moreHistory.toggleAttribute("disabled", historyLoading || !historyHasMore);
  }

  function renderNotifications() {
    clear(notificationList);
    if (!notificationUpdate || notificationUpdate.suppressed || !notificationUpdate.triggers.length) return;
    const triggers = [...notificationUpdate.triggers].sort((a, b) => priorityRank(a.priority) - priorityRank(b.priority));
    for (const trigger of triggers.slice(0, 3)) {
      const row = h("div", { class: "monitor-notification" },
        h("div", { class: "monitor-notification-copy" },
          h("b", { text: trigger.title }),
          trigger.detail ? h("span", { text: trigger.detail }) : "",
        ),
      );
      const target = safeNotificationTarget(trigger.target);
      if (target?.kind === "openCodex") {
        row.append(h("button", { class: "link-btn", text: "Open Codex", onclick: () => actions.openCodex() }));
      } else if (target?.kind === "openGitHub") {
        row.append(h("button", { class: "link-btn", text: "Open GitHub", onclick: () => {
          actions.runBridgeAction(Bridge.openUrl(target.url), "Could not open the GitHub notification target.");
        } }));
      }
      notificationList.append(row);
    }
  }

  function historyRow(record: BrokerHistoryRecord) {
    const sequence = record.sourceSequence == null ? "" : ` · sequence ${record.sourceSequence}`;
    return h("div", { class: "monitor-history-row" },
      dot(record.source === "codex_cli_observer" ? "#A78BFA" : "#A1A1AA", 5),
      h("span", { class: "monitor-history-kind", text: `${record.source} · ${record.recordKind}${sequence}` }),
      h("time", { text: formatObservedTime(record.observedAtUnixMs) }),
    );
  }

  return {
    el,
    sync() {
      refreshButton.toggleAttribute("disabled", statusLoading);
      renderSessions(projectSessions(BrokerView.snapshot));
      renderHistory();
      renderNotifications();
    },
    setNotificationUpdate(update: NotificationUpdate) {
      notificationUpdate = update;
      renderNotifications();
    },
    focus() {
      if (!historyLoaded && !historyLoading) void loadHistory(false);
    },
  };
}

function priorityRank(priority: NotificationUpdate["primary"]): number {
  return ({ waiting: 0, failure: 1, active: 2, completion: 3, idle: 4 })[priority];
}

function asTimestamp(value: unknown): number | null {
  return typeof value === "number" && Number.isFinite(value) ? value : null;
}
