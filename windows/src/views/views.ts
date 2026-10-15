// Island views — DOM ports of IslandViewContent.swift. Paddings, font sizes,
// colours and wording are copied from the Swift views so both platforms read
// identically.

import { h, svg, clear, dot } from "./dom";
import { ICONS } from "./icons";
import { Ticker } from "./ticker";
import { State, type AgentTask } from "../core/state";
import { washRGBA, type IslandViewName, type Wash } from "../core/layout";
import { capabilityRequestGeneration } from "../core/capability-guard.js";
import { createMiniBot, pruneMiniBots } from "../mochi/minibots";
import { buildPrompt } from "./chat";
import { buildChoose, buildUpload, buildUploading } from "./upload";
import { integrationHealth, renderIntegrationCard, type IntegrationCardHooks } from "./integrations";
import { buildMonitorView } from "./monitor";
import { buildGithubWorkflowCard } from "./github-workflow";
import { CodexPresentation, type CodexSessionPresentation } from "../core/codex-presentation";

export interface ViewActions {
  setView(v: IslandViewName): void;
  collapse(): void;
  setFocus(id: string): void;
  openTerminal(): void;
  /** The ↗ button: opens whatever the focused pill points at. */
  openTarget(): void;
  openUrl(url: string): void;
  decide(d: "allow" | "deny"): void;
  toggleSound(): void;
  setVolume(v: number): void;
  setAutoClose(seconds: number): void;
  openSettingsWindow(): void;
  openCodex(): void;
  runBridgeAction<T>(operation: Promise<T>, failureMessage: string): void;
  blip(): void;
}

export interface ViewHost {
  el: HTMLElement;
  sync(): void;
  /** Called when the view becomes active, for views with a text field. */
  focus?(): void;
  /** Called every frame while the view is on screen. */
  tick?(nowMs: number): void;
  setNotificationBadge?(count: number, label: string, priority: import("../core/notification-policy").NotificationPriority): void;
  setNotificationUpdate?(update: import("../core/notification-policy").NotificationUpdate): void;
}

// ── Shared pieces ─────────────────────────────────────────────────────────────

function card(wash: Wash, ...children: (Node | string)[]): HTMLElement {
  const el = h("div", { class: wash ? "card wash" : "card" }, ...children);
  if (wash) el.style.setProperty("--wash", washRGBA(wash));
  return el;
}

function btn(
  label: string,
  kind: "primary" | "secondary",
  onClick: () => void,
  kbd?: string,
): HTMLElement {
  return h(
    "button",
    { class: `btn ${kind}`, onclick: onClick },
    h("span", { text: label }),
    kbd ? h("span", { class: "kbd", text: kbd }) : null,
  );
}

/** AgentWho — coloured dot + task name + grey label. */
function agentWho(task: AgentTask | null, label: string): HTMLElement {
  const row = h("div", { class: "who-row" });
  if (task) {
    row.append(dot(task.color, 8), h("span", { class: "n", text: task.name }));
  }
  row.append(h("span", { text: label }));
  return row;
}

function stack(padLeft: number, padRight: number, ...children: Node[]): HTMLElement {
  const el = h("div", { class: "stack" }, ...children);
  el.style.padding = `4px ${padRight}px 4px ${padLeft}px`;
  return el;
}

// ── Header ────────────────────────────────────────────────────────────────────

export function buildHeader(actions: ViewActions): ViewHost {
  const tabHome = h("button", { class: "tab", title: "Overview", "aria-label": "Overview", onclick: () => go("overview") }, svg(ICONS.house, 13));
  const tabChat = h("button", { class: "tab", title: "Ask Codex", "aria-label": "Open Codex", onclick: () => go("prompt") }, svg(ICONS.bubble, 13));
  const tabDrop = h("button", { class: "tab", title: "Drop a file", "aria-label": "Open Codex to attach a file", onclick: () => go("upload") }, svg(ICONS.plus, 13));
  const historyBadge = h("span", { class: "tab-notification-badge", "aria-hidden": "true" });
  let badgeTimer: number | null = null;
  let badgeLabel = "Codex activity and history";
  const tabHistory = h("button", { class: "tab", title: badgeLabel, "aria-label": badgeLabel, onclick: () => {
    setNotificationBadge(0, "Codex activity and history", "idle");
    go("history");
  } }, svg(ICONS.doc, 13), historyBadge);

  function setNotificationBadge(count: number, label: string, priority: import("../core/notification-policy").NotificationPriority) {
    if (badgeTimer != null) window.clearTimeout(badgeTimer);
    badgeTimer = null;
    badgeLabel = count > 0 ? `${label}; ${count} new item${count === 1 ? "" : "s"}` : "Codex activity and history";
    historyBadge.textContent = count > 9 ? "9+" : count > 0 ? String(count) : "";
    historyBadge.style.display = count > 0 ? "grid" : "none";
    historyBadge.dataset.priority = count > 0 ? priority : "idle";
    tabHistory.title = badgeLabel;
    tabHistory.setAttribute("aria-label", badgeLabel);
    if (count > 0) badgeTimer = window.setTimeout(() => setNotificationBadge(0, "Codex activity and history", "idle"), 15_000);
  }

  const gearBtn = h("button", { title: "Settings", onclick: () => go("settings") }, svg(ICONS.gear, 14));
  const soundBtn = h("button", { title: "Mute", onclick: () => actions.toggleSound() }, svg(ICONS.speakerOn, 14));

  function go(v: IslandViewName) {
    actions.blip();
    if (v === "prompt" && !capabilityRequestGeneration(State.capabilities, "codex.managedSession")) {
      actions.openCodex();
      return;
    }
    if (v === "upload" && !capabilityRequestGeneration(State.capabilities, "codex.attachmentDelivery")) {
      actions.openCodex();
      return;
    }
    actions.setView(v);
  }

  const el = h(
    "div",
    { id: "header" },
    h("div", { class: "tabs" }, tabHome, tabChat, tabDrop, tabHistory),
    h("div", { class: "header-actions" }, gearBtn, soundBtn),
  );

  return {
    el,
    setNotificationBadge(count: number, label: string, priority: import("../core/notification-policy").NotificationPriority) { setNotificationBadge(count, label, priority); },
    sync() {
      const v = State.view;
      tabHome.classList.toggle("on", v === "overview" || v === "empty");
      tabChat.classList.toggle("on", v === "prompt");
      tabDrop.classList.toggle("on", v === "upload");
      tabHistory.classList.toggle("on", v === "history");
      tabChat.title = capabilityRequestGeneration(State.capabilities, "codex.managedSession")
        ? "Ask Codex"
        : "Companion chat is unavailable. Continue in Codex.";
      tabDrop.title = capabilityRequestGeneration(State.capabilities, "codex.attachmentDelivery")
        ? "Drop a file"
        : "File delivery is unavailable. Open Codex to attach files.";
      const chatAvailable = capabilityRequestGeneration(State.capabilities, "codex.managedSession") !== null;
      const attachmentAvailable = capabilityRequestGeneration(State.capabilities, "codex.attachmentDelivery") !== null;
      if (!chatAvailable) tabChat.title = "Companion chat is unavailable. Open Codex instead.";
      if (!attachmentAvailable) tabDrop.title = "Attachment delivery is unavailable. Open Codex to attach files.";
      gearBtn.classList.toggle("on", v === "settings");
      clear(gearBtn);
      gearBtn.append(svg(v === "settings" ? ICONS.gearFill : ICONS.gear, 14));
      clear(soundBtn);
      soundBtn.append(svg(State.settings.soundEnabled ? ICONS.speakerOn : ICONS.speakerOff, 14));
      el.style.opacity = v === "confused" ? "0" : "1";
    },
  };
}

// ── Overview ──────────────────────────────────────────────────────────────────

function buildOverview(actions: ViewActions): ViewHost {
  const ticker = new Ticker();
  const who = h("div", { class: "who" });
  const tickerBody = h("div", { class: "card-body" }, who, ticker.el);
  const leftBody = h("div", { class: "left-body" });
  const jump = h(
    "button",
    { class: "icon-btn jump", title: "Open Codex", "aria-label": "Open Codex", onclick: () => State.focusTask ? actions.openTarget() : actions.openCodex() },
    svg(ICONS.arrowUpRight, 8),
  );
  const left = card(null, leftBody, jump);
  const pills = h("div", { class: "pills" });
  const right = card(null, pills);

  const el = h("div", { class: "view overview" },
    h("div", { class: "left" }, left),
    h("div", { class: "right" }, right),
  );

  let pillIds = "";
  let detailOpen = false;
  let lastFocus: string | null = null;
  let mode: "ticker" | "card" | null = null;
  let cardKey = "";

  const hooks: IntegrationCardHooks = {
    get detailOpen() {
      return detailOpen;
    },
    openDetail() {
      detailOpen = true;
      cardKey = "";
      State.notify();
    },
    closeDetail() {
      detailOpen = false;
      cardKey = "";
      State.notify();
    },
    openSettings: () => actions.openSettingsWindow(),
    runBridgeAction: (operation, message) => actions.runBridgeAction(operation, message),
  };

  return {
    el,
    tick(nowMs: number) {
      if (mode === "ticker") ticker.tick(nowMs);
    },
    sync() {
      const task = State.focusTask;
      const codexSession = task ? null : CodexPresentation.selectedAvatar();
      const focusKey = task?.id ?? codexSession?.key ?? null;
      if (focusKey !== lastFocus) {
        lastFocus = focusKey;
        detailOpen = false;
        cardKey = "";
        mode = null;
      }

      // VS Code with a live Claude Code session keeps the ticker; every other
      // pill shows its own card, exactly like IntegrationCardView.
      const sessionActive =
        task?.id === "integration_claude" && (task.state !== "idle" || task.steps.length > 0);

      if (task && sessionActive) {
        if (mode !== "ticker") {
          clear(leftBody);
          leftBody.append(tickerBody);
          mode = "ticker";
          cardKey = "";
        }
        clear(who);
        who.append(
          dot(task.color, 7),
          h("span", { class: "name", text: task.name }),
          h("span", { class: "tool", text: task.source === "claudeCode" ? "Claude Code" : "n8n" }),
        );
        if (task.steps.length > 1) {
          who.append(h("span", {
            class: "count",
            text: `${Math.min(task.stepSequence, task.steps.length)}/${task.steps.length}`,
          }));
        }
        ticker.sync(task);
      } else if (task) {
        const info = State.integrations[task.id];
        const health = integrationHealth(task.id);
        const key = [
          task.id, detailOpen, task.state, task.steps.map((step) => step.text).join("|"),
          info?.loaded, info?.error, info?.configured, health?.connection, health?.lastError,
          JSON.stringify(info?.data ?? {}),
        ].join("~");
        if (key !== cardKey) {
          cardKey = key;
          mode = "card";
          clear(leftBody);
          leftBody.append(renderIntegrationCard(task, hooks));
        }
      } else if (codexSession) {
        const key = `codex-session:${codexSession.key}:${codexSession.state}:${codexSession.description}`;
        if (key !== cardKey) {
          cardKey = key;
          mode = "card";
          clear(leftBody);
          leftBody.append(
            h("div", { class: "card-body" },
              h("div", { class: "who" }, dot(codexSession.color, 7), h("span", { class: "name", text: codexSession.title }), h("span", { class: "tool", text: "Codex CLI" })),
              h("div", { class: "monitor-overview-summary", text: codexSession.description }),
              h("button", { class: "link-btn monitor-overview-link", text: "Activity and history…", onclick: () => actions.setView("history") }),
            ),
          );
        }
      } else {
        const key = "codex-monitor:empty";
        if (key !== cardKey) {
          cardKey = key;
          mode = "card";
          clear(leftBody);
          leftBody.append(
            h("div", { class: "card-body" },
              h("div", { class: "who" }, dot("#A78BFA", 7), h("span", { class: "name", text: "Codex sessions" })),
              h("div", { class: "monitor-overview-summary", text: "No Codex sessions observed yet." }),
              h("button", { class: "link-btn monitor-overview-link", text: "Activity and history…", onclick: () => actions.setView("history") }),
            ),
          );
        }
      }

      jump.style.display = detailOpen ? "none" : "";

      if (task) {
        const others = State.otherTasks.slice(0, 4);
        const pillKey = others.map((t) => `${t.id}:${t.pillBadge ?? ""}`).join("|");
        if (pillKey !== pillIds) {
          pillIds = pillKey;
          clear(pills);
          for (const t of others) pills.append(buildPill(t, actions));
          pruneMiniBots();
        }
      } else {
        const others = CodexPresentation.otherAvatars(4);
        const pillKey = `${CodexPresentation.selectedKey ?? "none"}:` + others.map((session) => `${session.key}:${session.state}`).join("|")
          + `:overflow-${CodexPresentation.overflowCount}`;
        if (pillKey !== pillIds) {
          pillIds = pillKey;
          clear(pills);
          for (const session of others) pills.append(buildSessionPill(session));
          if (CodexPresentation.overflowCount > 0) {
            pills.append(h("button", {
              class: "pill session-overflow",
              text: `+${CodexPresentation.overflowCount} more`,
              "aria-label": `${CodexPresentation.overflowCount} more Codex sessions; open activity history`,
              onclick: () => actions.setView("history"),
            }));
          }
          pruneMiniBots();
        }
      }
    },
  };
}

function buildPill(task: AgentTask, actions: ViewActions): HTMLElement {
  const label = task.id === "integration_claude" ? "VS Code" : task.name;
  const canvas = createMiniBot(task, 24);
  const pill = h(
    "div",
    { class: "pill", onclick: () => actions.setFocus(task.id) },
    canvas,
    h("span", { class: "lbl", text: label }),
  );
  pill.style.borderColor = `${task.color}24`;
  pill.addEventListener("mouseenter", () => {
    pill.style.background = `${task.color}2e`;
    pill.style.borderColor = `${task.color}8c`;
    pill.style.boxShadow = `0 2px 10px ${task.color}59`;
    (pill.querySelector(".lbl") as HTMLElement).style.color = lighten(task.color, 0.3);
  });
  pill.addEventListener("mouseleave", () => {
    pill.style.background = "";
    pill.style.borderColor = `${task.color}24`;
    pill.style.boxShadow = "";
    (pill.querySelector(".lbl") as HTMLElement).style.color = "";
  });

  if (task.pillBadge) {
    const colors = { approval: "#F5A524", finished: "#22C55E", error: "#F4505E" } as const;
    const icons = { approval: ICONS.bang, finished: ICONS.check, error: ICONS.xmark } as const;
    const inner = h("i", { style: `background:${colors[task.pillBadge]}` }, svg(icons[task.pillBadge], 6, { stroke: task.pillBadge === "finished" ? 3 : 0 }));
    const badge = h("div", { class: "pill-badge" }, inner);
    badge.style.boxShadow = `0 0 4px ${colors[task.pillBadge]}99`;
    pill.append(badge);
  }
  return pill;
}

function buildSessionPill(session: CodexSessionPresentation): HTMLElement {
  const canvas = createMiniBot({ id: session.key, color: session.color, state: session.state }, 24);
  const pill = h(
    "button",
    {
      class: "pill",
      type: "button",
      title: session.description,
      "aria-label": `Select ${session.title}. ${session.description}`,
      "aria-pressed": CodexPresentation.selectedKey === session.key ? "true" : "false",
      onclick: () => CodexPresentation.select(session.key),
    },
    canvas,
    h("span", { class: "lbl", text: session.title }),
  );
  pill.style.borderColor = `${session.color}24`;
  pill.addEventListener("mouseenter", () => {
    pill.style.background = `${session.color}2e`;
    pill.style.borderColor = `${session.color}8c`;
    pill.style.boxShadow = `0 2px 10px ${session.color}59`;
    (pill.querySelector(".lbl") as HTMLElement).style.color = lighten(session.color, 0.3);
  });
  pill.addEventListener("mouseleave", () => {
    pill.style.background = "";
    pill.style.borderColor = `${session.color}24`;
    pill.style.boxShadow = "";
    (pill.querySelector(".lbl") as HTMLElement).style.color = "";
  });
  return pill;
}

function lighten(hex: string, amount: number): string {
  const v = parseInt(hex.replace("#", ""), 16);
  const c = [(v >> 16) & 255, (v >> 8) & 255, v & 255].map((x) =>
    Math.min(255, Math.round(x + amount * 255)),
  );
  return `rgb(${c[0]},${c[1]},${c[2]})`;
}

// ── Empty ─────────────────────────────────────────────────────────────────────

function buildEmpty(actions: ViewActions): ViewHost {
  const body = h(
    "div",
    { class: "stack", style: "padding:0 18px 0 118px;flex-direction:row;align-items:center;gap:16px" },
    h(
      "div",
      { style: "display:flex;flex-direction:column;gap:5px" },
      h("div", { class: "title", text: "Nothing running right now." }),
      h("div", { class: "sub", text: "Open Codex to start a session or attach files." }),
    ),
    h("div", { class: "grow" }),
    btn("Continue in Codex", "primary", () => actions.openCodex()),
  );
  return { el: h("div", { class: "view" }, card(null, body)), sync() {} };
}

// ── Approval ──────────────────────────────────────────────────────────────────

function buildApproval(actions: ViewActions): ViewHost {
  const who = h("div");
  const code = h("div", { class: "code" });
  const feedback = h("div", { class: "sub" });
  const row = h("div", { class: "actions" });
  const el = h("div", { class: "view" }, card("amber", stack(116, 16, who, code, feedback, row)));
  let rowKey = "";
  return {
    el,
    sync() {
      clear(who);
      who.append(agentWho(State.focusTask, "needs permission"));
      // The whole point of approving here rather than in the terminal: this line
      // is the command, the file path or the URL being authorised, not just the
      // name of the tool asking.
      code.textContent = State.pendingApproval?.command || State.pendingApproval?.tool || "…";
      feedback.textContent = State.pendingApproval ? State.noteMessage ?? "" : "";
      feedback.style.display = feedback.textContent ? "" : "none";
      // Two buttons, built once. Rebuilding them between a mouse-down and a
      // mouse-up would swallow the click, and there is nothing left to vary:
      // "Always" is gone until the remembered-rules list exists to back it.
      if (rowKey === "built") return;
      rowKey = "built";
      clear(row);
      row.append(
        btn("Deny", "secondary", () => actions.decide("deny"), "N"),
        btn("Allow", "primary", () => actions.decide("allow"), "Y"),
      );
    },
  };
}

// ── Question ──────────────────────────────────────────────────────────────────

function buildQuestion(): ViewHost {
  const who = h("div");
  const title = h("div", { class: "title" });
  const row = h("div", { class: "actions" });
  const el = h("div", { class: "view" }, card("cyan", stack(116, 16, who, title, row)));
  return {
    el,
    sync() {
      clear(who);
      who.append(agentWho(State.focusTask, "Claude Code is asking a question"));
      const task = State.focusTask;
      title.textContent = task?.steps.at(-1)?.text ?? "Claude needs an answer.";
      clear(row);
      row.append(h("div", { class: "sub", text: "Answer in your terminal — Anti-Scrolling-Notch can't reply for you yet." }));
    },
  };
}

// ── Error ─────────────────────────────────────────────────────────────────────

function buildError(actions: ViewActions): ViewHost {
  const who = h("div");
  const title = h("div", { class: "title", text: "Workflow stopped." });
  const detail = h("div", { class: "detail" });
  const row = h("div", { class: "actions" },
    btn("Retry", "primary", () => actions.setView(State.defaultView())),
    btn("Open in n8n", "secondary", () => actions.openUrl("")),
  );
  const el = h("div", { class: "view" }, card("red", stack(116, 16, who, title, detail, row)));
  return {
    el,
    sync() {
      const task = State.focusTask;
      clear(who);
      who.append(agentWho(task, task?.source === "n8n" ? "n8n" : "Claude Code"));
      title.textContent = task?.source === "n8n" ? "Workflow stopped." : "Session stopped on an error.";
      detail.textContent = task?.steps.at(-1)?.text ?? "No detail available.";
    },
  };
}

// ── Finished ──────────────────────────────────────────────────────────────────

function buildFinished(actions: ViewActions): ViewHost {
  const who = h("div");
  const title = h("div", { class: "title" });
  const row = h("div", { class: "actions" },
    btn("Open terminal", "primary", () => actions.openTerminal()),
    btn("OK", "secondary", () => actions.collapse()),
  );
  const el = h("div", { class: "view" }, card("green", stack(116, 16, who, title, row)));
  return {
    el,
    sync() {
      clear(who);
      who.append(agentWho(State.focusTask, "Claude Code finished"));
      title.textContent = State.focusTask?.steps.at(-1)?.text ?? "Session finished";
    },
  };
}

// ── Confused ──────────────────────────────────────────────────────────────────

function buildConfused(): ViewHost {
  const body = h(
    "div",
    { class: "stack", style: "padding:0 18px 0 128px" },
    h("div", { class: "title", text: "Too many hits at once." }),
    h("div", { class: "sub", text: "Give me a sec — back to work in three seconds." }),
  );
  return { el: h("div", { class: "view" }, card("pink", body)), sync() {} };
}

// ── Note ──────────────────────────────────────────────────────────────────────

function buildNote(): ViewHost {
  const title = h("div", { class: "title" });
  const el = h("div", { class: "view" }, card(null, h("div", { class: "stack", style: "padding:0 18px 0 98px" }, title)));
  return {
    el,
    sync() {
      title.textContent = State.noteMessage ?? "";
    },
  };
}

// ── In-island settings ────────────────────────────────────────────────────────

function buildSettings(actions: ViewActions): ViewHost {
  const soundSwitch = h("button", { class: "switch", onclick: () => actions.toggleSound() });
  const volume = h("input", {
    type: "range", min: "0", max: "0.2", step: "0.005",
    oninput: (e: Event) => actions.setVolume(Number((e.target as HTMLInputElement).value)),
  }) as HTMLInputElement;
  const autoLabel = h("span", {});
  const segButtons = [10, 15, 30].map((s) =>
    h("button", { onclick: () => actions.setAutoClose(s) }, `${s}s`),
  );
  const claudeBadge = h("span", { class: "status-badge" });
  const apiBadge = h("span", { class: "status-badge" });

  const rows = h(
    "div",
    { class: "settings-rows" },
    h("div", { class: "settings-row" }, soundSwitch, h("span", { text: "Sound" }), volume),
    h(
      "div",
      { class: "settings-row" },
      svg(ICONS.timer, 12),
      autoLabel,
      h("div", { class: "seg" }, ...segButtons),
    ),
    h(
      "div",
      { class: "settings-row", style: "gap:14px" },
      claudeBadge,
      apiBadge,
      h("div", { class: "grow" }),
      h("button", {
        class: "link-btn",
        style: "color:#8e939c;font-size:11.5px",
        text: "Settings…",
        onclick: () => actions.openSettingsWindow(),
      }),
    ),
  );

  const el = h("div", { class: "view" },
    card(null, h("div", { class: "stack", style: "padding:14px 16px 14px 84px" }, rows)));

  return {
    el,
    sync() {
      const s = State.settings;
      soundSwitch.classList.toggle("on", s.soundEnabled);
      volume.value = String(s.soundVolume);
      volume.style.opacity = s.soundEnabled ? "1" : "0.4";
      autoLabel.textContent = `Auto-close · ${Math.round(s.autoCloseInterval)}s`;
      segButtons.forEach((b, i) => b.classList.toggle("on", s.autoCloseInterval === [10, 15, 30][i]));
      clear(claudeBadge);
      claudeBadge.append(
        dot(s.hooksInstalled ? "#22C55E" : "#F4505E", 6),
        h("span", { text: "Claude Code" }),
      );
      clear(apiBadge);
      apiBadge.append(dot("#F4505E", 6), h("span", { text: "API" }));
    },
  };
}

// ── Placeholders filled in later stages ───────────────────────────────────────

function buildPlaceholder(title: string, sub: string): ViewHost {
  const body = h(
    "div",
    { class: "stack", style: "padding:0 18px 0 118px" },
    h("div", { class: "title", text: title }),
    h("div", { class: "sub", text: sub }),
  );
  return { el: h("div", { class: "view" }, card(null, body)), sync() {} };
}

// ── Registry ──────────────────────────────────────────────────────────────────

export function buildViews(
  actions: ViewActions,
  onChatHeightChange: () => void,
): Map<IslandViewName, ViewHost> {
  const map = new Map<IslandViewName, ViewHost>();
  map.set("overview", buildOverview(actions));
  const monitor = buildMonitorView(actions);
  monitor.el.append(buildGithubWorkflowCard());
  map.set("history", monitor);
  map.set("empty", buildEmpty(actions));
  map.set("approval", buildApproval(actions));
  map.set("question", buildQuestion());
  map.set("error", buildError(actions));
  map.set("finished", buildFinished(actions));
  map.set("confused", buildConfused());
  map.set("note", buildNote());
  map.set("settings", buildSettings(actions));
  map.set("prompt", buildPrompt(onChatHeightChange));
  map.set("upload", buildUpload());
  map.set("uploading", buildUploading());
  map.set("choose", buildChoose(actions));
  // Not in the Windows v1: sending a file by email, window attach + web result.
  map.set("mail", buildPlaceholder("Sending by email isn't in this version.", ""));
  map.set("searching", buildPlaceholder("Claude is searching…", ""));
  map.set("result", buildPlaceholder("Result", ""));
  return map;
}
