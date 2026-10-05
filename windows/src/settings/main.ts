// Settings window — the place where anything that writes to disk is confirmed.

import "./settings.css";
import {
  Bridge, onEvent, reportBridgeFailure, safeBridgeFailure,
  type CodexHookStatus, type HookStatus, type MonitorOption, type ToggleShortcutStatus,
} from "../core/bridge";
import { Motion } from "../core/motion";
import { DEFAULT_SETTINGS, type Settings } from "../core/state";
import { h, clear } from "../views/dom";

let settings: Settings = { ...DEFAULT_SETTINGS };
let version = "";
let monitors: MonitorOption[] = [];
let shortcutStatus: ToggleShortcutStatus = { active: null, error: null };

const root = document.getElementById("settings-root")!;
const pageNotice = h("div", { class: "notice", hidden: true });

function showPageNotice(message: string, kind: "warn" | "err" = "err") {
  pageNotice.className = `notice ${kind}`;
  pageNotice.textContent = message;
  pageNotice.hidden = false;
}

function clearPageNotice(message?: string) {
  if (message && pageNotice.textContent !== message) return;
  pageNotice.textContent = "";
  pageNotice.hidden = true;
}

async function save() {
  try {
    await Bridge.saveSettings(settings);
    Motion.setReducedMotion(settings.reducedMotion);
    clearPageNotice("Preferences could not be saved. This change may not persist.");
  } catch (error) {
    reportBridgeFailure(error, "settings save");
    showPageNotice("Preferences could not be saved. This change may not persist.");
  }
}

// ── Reusable bits ─────────────────────────────────────────────────────────────

function toggle(on: boolean, onChange: (v: boolean) => void): HTMLElement {
  const el = h("button", { class: on ? "switch on" : "switch", "aria-pressed": on });
  el.addEventListener("click", () => {
    const next = !el.classList.contains("on");
    el.classList.toggle("on", next);
    onChange(next);
  });
  return el;
}

function statusDot(ok: boolean | null): HTMLElement {
  const color = ok === null ? "#f5a524" : ok ? "#22c55e" : "#f4505e";
  return h("i", { class: "dot", style: `background:${color}` });
}

function renderDiff(text: string): HTMLElement {
  const box = h("div", { class: "diff" });
  for (const line of text.split("\n")) {
    const cls = line.startsWith("+") ? "add" : line.startsWith("-") ? "del" : "ctx";
    box.append(h("div", { class: cls, text: line }));
  }
  return box;
}

// ── Claude Code section ───────────────────────────────────────────────────────

function claudeSection(initialStatus: HookStatus | null): HTMLElement {
  let status = initialStatus;
  const body = h("div", { style: "display:flex;flex-direction:column;gap:12px" });
  const section = h(
    "section",
    {},
    h("h2", {}, statusDot(status?.installed ?? null), h("span", { text: "Claude Code" })),
    body,
  );

  const rebuild = async () => {
    let fresh: HookStatus | null;
    try {
      fresh = await Bridge.hooksStatus();
    } catch (error) {
      reportBridgeFailure(error, "hook status refresh");
      showPageNotice("Hook status could not be refreshed; the last known status is shown.", "warn");
      return;
    }
    if (!fresh) {
      showPageNotice("Hook status is unavailable in this preview.", "warn");
      return;
    }
    clearPageNotice("Hook status could not be refreshed; the last known status is shown.");
    status = fresh;
    clear(body);
    draw();
    const head = section.querySelector("h2")!;
    clear(head);
    head.append(statusDot(status.installed), h("span", { text: "Claude Code" }));
  };

  function draw() {
    if (!status) {
      body.append(
        h("div", { class: "hint", text: "Hook status could not be loaded. Reopen Settings to try again." }),
        h("div", { class: "row" }, h("label", { text: "Hooks" }), statusDot(null)),
      );
      return;
    }
    body.append(
      h("div", {
        class: "hint",
        text: status.installed
          ? "Anti-Scrolling-Notch is hooked into your Claude Code sessions. Tool calls, questions and permission requests show up in the island, and you can answer them there."
          : "Install the hooks to see your Claude Code sessions in the island and approve permissions without leaving what you are doing.",
      }),
      h("div", { class: "row" },
        h("label", { text: "settings.json" }),
        h("span", { class: "path", text: status.settingsPath }),
      ),
      h("div", { class: "row" },
        h("label", { text: "Relay" }),
        h("span", { class: "path", text: status.hookPath }),
        statusDot(status.hookReady),
      ),
    );

    if (!status.hookReady) {
      body.append(h("div", {
        class: "notice warn",
        text: "anti-scrolling-notch-hook.exe is not in place yet. Restart Anti-Scrolling-Notch; if it still fails, build it with `cargo build -p anti-scrolling-notch-hook`.",
      }));
    }

    const actions = h("div", { class: "row" });
    const install = h("button", {
      class: "primary",
      text: status.installed ? "Reinstall hooks…" : "Install hooks…",
      onclick: () => showPreview(true),
    });
    // Writing hook commands that point at a relay which isn't there would give
    // every Claude Code session a broken hook and nothing to show for it.
    if (!status.hookReady) {
      install.disabled = true;
      install.title = "The relay isn't installed yet.";
    }
    actions.append(install);
    if (status.installed) {
      actions.append(h("button", {
        class: "danger",
        text: "Uninstall hooks…",
        onclick: () => showPreview(false),
      }));
    }
    body.append(actions);
  }

  async function showPreview(install: boolean) {
    let preview;
    try {
      preview = await Bridge.hooksPreview(install);
    } catch (err) {
      // An unreadable or invalid settings.json stops here rather than being
      // treated as empty and written over.
      clear(body);
      reportBridgeFailure(err, "hook preview");
      body.append(
        h("div", { class: "notice err", text: safeBridgeFailure(err, "Could not load the hook changes.") }),
        h("div", { class: "row" }, h("button", {
          text: "Back",
          onclick: () => { clear(body); draw(); },
        })),
      );
      return;
    }
    if (!preview) return;
    clear(body);
    body.append(
      h("div", {
        class: "hint",
        text: install
          ? "This is exactly what will change in your settings.json. Your own hooks are left untouched."
          : "This removes Anti-Scrolling-Notch's entries only. Existing Coucou and your other hooks are left untouched.",
      }),
      renderDiff(preview.diff),
      h("div", { class: "row" },
        h("span", { class: "path", text: `Backup → ${preview.backup}` }),
      ),
    );
    const confirm = h("button", {
      class: install ? "primary" : "danger",
      text: install ? "Back up and write" : "Back up and remove",
    });
    confirm.addEventListener("click", async () => {
      confirm.disabled = true;
      try {
        const backup = await Bridge.hooksApply(install, preview.fingerprint);
        clear(body);
        body.append(h("div", {
          class: "notice ok",
          text: `Done. Previous settings saved as ${backup}. Open a new Claude Code session to pick the hooks up.`,
        }));
        window.setTimeout(() => void rebuild(), 2600);
      } catch (err) {
        reportBridgeFailure(err, "hook update");
        confirm.disabled = false;
        body.append(h("div", { class: "notice err", text: `Could not write: ${safeBridgeFailure(err, "the hook settings")}` }));
      }
    });
    body.append(h("div", { class: "row" }, confirm, h("button", {
      text: "Cancel",
      onclick: () => { clear(body); draw(); },
    })));
  }

  draw();
  return section;
}

// ── Codex CLI hook configuration ────────────────────────────────────────────

function codexHooksSection(initialStatus: CodexHookStatus | null): HTMLElement {
  let status = initialStatus;
  const body = h("div", { style: "display:flex;flex-direction:column;gap:12px" });
  const section = h(
    "section",
    {},
    h("h2", {}, h("span", { text: "Codex CLI hooks" })),
    body,
  );

  const rebuild = async () => {
    try {
      status = await Bridge.codexHooksStatus();
      clearPageNotice("Codex hook status could not be refreshed; the last known status is shown.");
      clear(body);
      draw();
    } catch (error) {
      reportBridgeFailure(error, "Codex hook status refresh");
      showPageNotice("Codex hook status could not be refreshed; the last known status is shown.", "warn");
    }
  };

  function draw() {
    if (!status) {
      body.append(
        h("div", { class: "hint", text: "Codex hook status could not be loaded. Reopen Settings to try again." }),
      );
      return;
    }

    const configuredCount = status.configuredEvents.length;
    const versionLine = status.cliSupported
      ? `Supported CLI detected: ${status.cliVersion}`
      : `Codex CLI ${status.supportedCliVersion} was not verified; installation is disabled.`;
    body.append(
      h("div", {
        class: "hint",
        text: "This configures only the four captured Codex CLI lifecycle hooks. The observer reads the event name and discards other hook fields; it does not control Codex.",
      }),
      h("div", { class: "row" },
        h("label", { text: "CLI support" }),
        h("span", { class: "path", text: versionLine }),
      ),
      h("div", { class: "row" },
        h("label", { text: "Configured events" }),
        h("span", { text: `${configuredCount}/4` }),
      ),
      h("div", { class: "row" },
        h("label", { text: "hooks.json" }),
        h("span", { class: "path", text: status.settingsPath }),
      ),
      h("div", { class: "row" },
        h("label", { text: "Observer relay" }),
        h("span", { class: "path", text: status.hookPath }),
        h("span", { text: status.hookReady ? "Ready" : "Missing" }),
      ),
      h("div", {
        class: "notice warn",
        text: "After installing, review and trust this hook in Codex CLI with /hooks. A configured file does not mean the hook is trusted or that a session is connected.",
      }),
    );

    const install = h("button", {
      class: "primary",
      text: configuredCount === 4 ? "Preview hook update…" : "Preview hook install…",
      onclick: () => showPreview(true),
    });
    install.disabled = !status.cliSupported || !status.hookReady;
    install.title = !status.cliSupported
      ? `Only Codex CLI ${status.supportedCliVersion} is supported by the captured hook contract.`
      : !status.hookReady ? "The observer relay is not installed yet." : "";
    const actions = h("div", { class: "row" }, install);
    if (configuredCount > 0) {
      actions.append(h("button", {
        class: "danger",
        text: "Preview hook removal…",
        onclick: () => showPreview(false),
      }));
    }
    body.append(actions);
  }

  async function showPreview(install: boolean) {
    let preview;
    try {
      preview = await Bridge.codexHooksPreview(install);
    } catch (error) {
      clear(body);
      reportBridgeFailure(error, "Codex hook preview");
      body.append(
        h("div", { class: "notice err", text: safeBridgeFailure(error, "Could not load the Codex hook changes.") }),
        h("div", { class: "row" }, h("button", {
          text: "Back",
          onclick: () => { clear(body); draw(); },
        })),
      );
      return;
    }
    clear(body);
    body.append(
      h("div", {
        class: "hint",
        text: install
          ? "Review this exact change to your user-level Codex hooks.json. Other hook handlers and settings are preserved."
          : "This removes only Anti-Scrolling-Notch's exact observer command; other Codex handlers are preserved.",
      }),
      renderDiff(preview.diff),
      h("div", { class: "row" }, h("span", {
        class: preview.backup ? "path" : "hint",
        text: preview.backup
          ? `Backup → ${preview.backup}`
          : "No backup will be created because hooks.json does not already exist or this preview makes no change.",
      })),
    );
    const confirm = h("button", {
      class: install ? "primary" : "danger",
      text: install
        ? preview.backup ? "Back up and write" : "Write hooks.json"
        : preview.backup ? "Back up and remove" : "Remove hooks",
    });
    confirm.addEventListener("click", async () => {
      confirm.disabled = true;
      try {
        const result = await Bridge.codexHooksApply(install, preview.fingerprint, preview.backup);
        clear(body);
        const successText = result.startsWith("No change;")
          ? result
          : result.startsWith("Created hooks.json.")
            ? `${result} Open Codex CLI and use /hooks to review and trust this hook before it can run.`
            : install
              ? `Saved a backup at ${result}. Open Codex CLI and use /hooks to review and trust this hook before it can run.`
              : `Saved a backup at ${result}. The owned observer hooks were removed.`;
        body.append(h("div", {
          class: "notice ok",
          text: successText,
        }));
        window.setTimeout(() => void rebuild(), 2600);
      } catch (error) {
        reportBridgeFailure(error, "Codex hook update");
        confirm.disabled = false;
        body.append(h("div", { class: "notice err", text: `Could not write: ${safeBridgeFailure(error, "Codex hooks.json")}` }));
      }
    });
    body.append(h("div", { class: "row" }, confirm, h("button", {
      text: "Cancel",
      onclick: () => { clear(body); draw(); },
    })));
  }

  draw();
  return section;
}

// ── Claude API section ────────────────────────────────────────────────────────

const MODELS: [string, string][] = [
  ["claude-opus-5", "Claude Opus 5"],
  ["claude-sonnet-5", "Claude Sonnet 5"],
  ["claude-haiku-4-5", "Claude Haiku 4.5"],
];

function apiSection(hasKey: boolean | null): HTMLElement {
  let keyPresent = hasKey;
  const dot = statusDot(keyPresent);
  const state = h("span", {
    class: "hint",
    text: keyPresent === null
      ? "Credential status unavailable."
      : keyPresent ? "Key saved in the Windows Credential Manager." : "No key yet — the chat needs one.",
  });

  const field = h("input", {
    type: "password",
    placeholder: keyPresent === true ? "••••••••••••  (stored)" : "sk-ant-...",
    style: "flex:1 1 auto;min-width:0",
    autocomplete: "off",
    spellcheck: "false",
  }) as HTMLInputElement;

  const saveBtn = h("button", { class: "primary", text: "Save key" });
  const clearBtn = h("button", { class: "danger", text: "Remove" });
  const feedback = h("div", {});
  const renderStatus = () => {
    dot.style.background = keyPresent === null ? "#f5a524" : keyPresent ? "#22c55e" : "#f4505e";
    state.textContent = keyPresent === null
      ? "Credential status unavailable."
      : keyPresent
        ? "Key saved in the Windows Credential Manager."
        : "No key yet — the chat needs one.";
    field.placeholder = keyPresent === true ? "••••••••••••  (stored)" : "sk-ant-...";
    clearBtn.style.display = keyPresent === true ? "" : "none";
  };

  async function refresh() {
    try {
      const present = await Bridge.secretPresent("anthropic-api-key");
      if (present !== null) keyPresent = present;
    } catch (error) {
      reportBridgeFailure(error, "chat credential status");
      showPageNotice("Credential status could not be checked; the last confirmed value is shown when available.", "warn");
    }
    renderStatus();
  }

  saveBtn.addEventListener("click", async () => {
    const value = field.value.trim();
    if (!value) return;
    clear(feedback);
    try {
      await Bridge.secretSet("anthropic-api-key", value);
      keyPresent = true;
      renderStatus();
      field.value = "";
      feedback.append(h("div", { class: "notice ok", text: "Saved. It never touches disk." }));
      await refresh();
    } catch (err) {
      reportBridgeFailure(err, "chat credential save");
      feedback.append(h("div", { class: "notice err", text: `Could not save: ${safeBridgeFailure(err, "the key")}` }));
    }
  });

  clearBtn.addEventListener("click", async () => {
    clear(feedback);
    try {
      await Bridge.secretClear("anthropic-api-key");
      keyPresent = false;
      renderStatus();
      feedback.append(h("div", { class: "notice ok", text: "Key removed." }));
      await refresh();
    } catch (err) {
      reportBridgeFailure(err, "chat credential removal");
      feedback.append(h("div", { class: "notice err", text: `Could not remove: ${safeBridgeFailure(err, "the key")}` }));
    }
  });

  const model = h("select", {}) as HTMLSelectElement;
  for (const [id, label] of MODELS) model.append(h("option", { value: id, text: label }));
  if (!MODELS.some(([id]) => id === settings.model)) {
    model.append(h("option", { value: settings.model, text: settings.model }));
  }
  model.value = settings.model;
  model.addEventListener("change", () => {
    settings.model = model.value;
    void save();
  });

  renderStatus();

  return h(
    "section",
    {},
    h("h2", {}, dot, h("span", { text: "Claude" })),
    state,
    h("div", { class: "row" }, h("label", { text: "API key" }), field, saveBtn, clearBtn),
    h("div", { class: "row" }, h("label", { text: "Model" }), model),
    feedback,
  );
}

// ── Integrations section ──────────────────────────────────────────────────────

interface IntegrationDef {
  id: string;
  name: string;
  color: string;
  /** Credential Manager keys, in the order they are shown. */
  fields: { key: string; label: string; placeholder: string; secret: boolean }[];
}

const INTEGRATIONS: IntegrationDef[] = [
  { id: "integration_stripe", name: "Stripe", color: "#0570DE",
    fields: [{ key: "stripe-api-key", label: "Secret key", placeholder: "sk_live_…", secret: true }] },
  { id: "integration_github", name: "GitHub", color: "#F4505E",
    fields: [{ key: "github-token", label: "Token", placeholder: "ghp_…", secret: true }] },
  { id: "integration_vercel", name: "Vercel", color: "#7C5CFF",
    fields: [{ key: "vercel-token", label: "Token", placeholder: "…", secret: true }] },
  { id: "integration_n8n", name: "n8n", color: "#F29B38",
    fields: [
      { key: "n8n-url", label: "Instance URL", placeholder: "https://n8n.example.com", secret: false },
      { key: "n8n-api-key", label: "API key", placeholder: "…", secret: true },
    ] },
  { id: "integration_resend", name: "Resend", color: "#22C55E",
    fields: [{ key: "resend-api-key", label: "API key", placeholder: "re_…", secret: true }] },
  { id: "integration_notion", name: "Notion", color: "#8C8C8C",
    fields: [{ key: "notion-api-key", label: "Integration token", placeholder: "ntn_…", secret: true }] },
  { id: "integration_calcom", name: "Cal.com", color: "#C9956A",
    fields: [{ key: "calcom-api-key", label: "API key", placeholder: "cal_…", secret: true }] },
];

const MAX_ACTIVE = 4;

function integrationsSection(present: Record<string, boolean | null>): HTMLElement {
  const note = h("div", { class: "hint" });
  const list = h("div", { style: "display:flex;flex-direction:column;gap:14px" });

  function updateNote() {
    const used = settings.activeIntegrations.length;
    note.textContent = `Pick up to ${MAX_ACTIVE} pills to show next to Mochi — ${used}/${MAX_ACTIVE} in use. Keys are stored in the Windows Credential Manager, never on disk.`;
  }

  for (const def of INTEGRATIONS) {
    const active = settings.activeIntegrations.includes(def.id);
    const sw = h("button", { class: active ? "switch on" : "switch" });
    sw.addEventListener("click", () => {
      const on = settings.activeIntegrations.includes(def.id);
      if (on) {
        settings.activeIntegrations = settings.activeIntegrations.filter((x) => x !== def.id);
      } else {
        if (settings.activeIntegrations.length >= MAX_ACTIVE) return;
        settings.activeIntegrations = [...settings.activeIntegrations, def.id];
      }
      sw.classList.toggle("on", !on);
      updateNote();
      void save();
    });

    const rows = h("div", { style: "display:flex;flex-direction:column;gap:6px;flex:1 1 auto;min-width:0" });
    for (const field of def.fields) {
      const input = h("input", {
        type: field.secret ? "password" : "text",
        placeholder: present[field.key] ? "••••••••  (stored)" : field.placeholder,
        autocomplete: "off",
        spellcheck: "false",
        style: "flex:1 1 auto;min-width:0",
      }) as HTMLInputElement;
      const saveBtn = h("button", { text: "Save" });
      const dotEl = statusDot(present[field.key] ?? null);
      saveBtn.addEventListener("click", async () => {
        const value = input.value.trim();
        try {
          await Bridge.secretSet(field.key, value);
          present[field.key] = value.length > 0;
          input.value = "";
          input.placeholder = value ? "••••••••  (stored)" : field.placeholder;
          dotEl.style.background = value ? "#22c55e" : "#f4505e";
        } catch (error) {
          reportBridgeFailure(error, "integration credential save");
          const lastKnown = present[field.key] ?? null;
          dotEl.style.background = lastKnown === null ? "#f5a524" : lastKnown ? "#22c55e" : "#f4505e";
          showPageNotice(`Integration credential save could not be confirmed: ${safeBridgeFailure(error, "check the status before relying on it.")}`);
        }
      });
      rows.append(
        h("div", { class: "row" },
          h("label", { style: "min-width:104px", text: field.label }),
          input, saveBtn, dotEl,
        ),
      );
    }

    list.append(
      h("div", { style: "display:flex;gap:12px;align-items:flex-start" },
        h("div", { style: "display:flex;align-items:center;gap:8px;min-width:132px;padding-top:4px" },
          sw,
          h("i", { class: "dot", style: `background:${def.color}` }),
          h("span", { style: "font-size:12.5px", text: def.name }),
        ),
        rows,
      ),
    );
  }

  updateNote();
  return h("section", {}, h("h2", {}, h("span", { text: "Integrations" })), note, list);
}

// ── General section ───────────────────────────────────────────────────────────

function generalSection(): HTMLElement {
  const volume = h("input", {
    type: "range", min: "0", max: "0.2", step: "0.005",
    value: String(settings.soundVolume),
  }) as HTMLInputElement;
  volume.addEventListener("input", () => {
    settings.soundVolume = Number(volume.value);
    void save();
  });

  const autoClose = h("input", {
    type: "number", min: "5", max: "120", step: "1",
    value: String(Math.round(settings.autoCloseInterval)),
    style: "width:72px",
  }) as HTMLInputElement;
  autoClose.addEventListener("change", () => {
    settings.autoCloseInterval = Math.max(5, Math.min(120, Number(autoClose.value) || 15));
    autoClose.value = String(settings.autoCloseInterval);
    void save();
  });

  const screen = h("select", {}) as HTMLSelectElement;
  const refreshScreenOptions = () => {
    const selection = settings.screen;
    clear(screen);
    screen.append(
      h("option", { value: "primary", text: "Main display" }),
      h("option", { value: "cursor", text: "Display under the cursor" }),
    );
    for (const monitor of monitors) {
      screen.append(h("option", {
        value: `monitor:${monitor.id}`,
        text: `${monitor.label} · ${Math.round(monitor.scale * 100)}%`,
      }));
    }
    if (selection.startsWith("monitor:") && !monitors.some((m) => `monitor:${m.id}` === selection)) {
      screen.append(h("option", {
        value: selection,
        text: "Saved display unavailable — using main display",
        disabled: true,
      }));
    }
    screen.value = selection;
  };
  refreshScreenOptions();
  screen.addEventListener("change", () => {
    settings.screen = screen.value;
    void save();
  });

  void onEvent<null>("screen-changed", () => {
    void (async () => {
      try {
        const available = await Bridge.monitorOptions();
        if (available) {
          monitors = available;
          refreshScreenOptions();
        }
      } catch (error) {
        reportBridgeFailure(error, "monitor option refresh");
        showPageNotice("Display options could not be refreshed; the saved selection is unchanged.", "warn");
      }
    })();
  });

  const edgeOffset = h("input", {
    type: "number", min: "0", max: "120", step: "1",
    value: String(settings.edgeOffset),
    style: "width:76px",
  }) as HTMLInputElement;
  edgeOffset.addEventListener("change", () => {
    settings.edgeOffset = Math.max(0, Math.min(120, Number(edgeOffset.value) || 0));
    edgeOffset.value = String(settings.edgeOffset);
    void save();
  });

  const shortcut = h("input", {
    type: "text",
    value: settings.toggleShortcut,
    placeholder: "CommandOrControl+Alt+Shift+Space",
    autocomplete: "off",
    spellcheck: "false",
    style: "width:300px;max-width:100%",
  }) as HTMLInputElement;
  const shortcutFeedback = h("div", { class: "notice" });
  const updateShortcutFeedback = () => {
    if (shortcutStatus.error) {
      shortcutFeedback.className = "notice warn";
      shortcutFeedback.textContent = shortcutStatus.error;
    } else if (shortcutStatus.active) {
      shortcutFeedback.className = "notice ok";
      shortcutFeedback.textContent = `Active: ${shortcutStatus.active}`;
    } else {
      shortcutFeedback.className = "notice warn";
      shortcutFeedback.textContent = "No global shortcut is active.";
    }
  };
  updateShortcutFeedback();
  const applyShortcut = h("button", { class: "primary", text: "Apply" });
  applyShortcut.addEventListener("click", async () => {
    applyShortcut.disabled = true;
    const previous = shortcutStatus.active;
    try {
      shortcutStatus = await Bridge.setToggleShortcut(shortcut.value);
      settings.toggleShortcut = shortcut.value.trim();
      shortcut.value = settings.toggleShortcut;
    } catch (err) {
      const fallback = previous ? ` ${previous} remains active.` : " No global shortcut is active.";
      reportBridgeFailure(err, "toggle shortcut");
      shortcutStatus.error = `${safeBridgeFailure(err, "Could not update the shortcut.")}${fallback}`;
    } finally {
      updateShortcutFeedback();
      applyShortcut.disabled = false;
    }
  });

  return h(
    "section",
    {},
    h("h2", {}, h("span", { text: "General" })),
    h("div", { class: "row" },
      h("label", { text: "Sound" }),
      toggle(settings.soundEnabled, (v) => { settings.soundEnabled = v; void save(); }),
      volume,
    ),
    h("div", { class: "row" },
      h("label", { text: "Reduce motion" }),
      toggle(settings.reducedMotion, (v) => { settings.reducedMotion = v; void save(); }),
      h("span", { class: "hint", text: "Settle decorative animation; keep status and sounds." }),
    ),
    h("div", { class: "row" },
      h("label", { text: "Save turn text" }),
      toggle(settings.retainHistoryContent, (v) => { settings.retainHistoryContent = v; void save(); }),
      h("span", { class: "hint", text: "Retain redacted summaries/plans when a supported source provides them. Off by default." }),
    ),
    h("div", { class: "row" },
      h("label", { text: "Auto-close" }),
      autoClose,
      h("span", { class: "hint", text: "seconds after you leave the island" }),
    ),
    h("div", { class: "row" },
      h("label", { text: "Island lives on" }),
      screen,
    ),
    h("div", { class: "row" },
      h("label", { text: "Top edge offset" }),
      edgeOffset,
      h("span", { class: "hint", text: "logical pixels (0–120)" }),
    ),
    h("div", { class: "row" },
      h("label", { text: "Toggle shortcut" }),
      shortcut,
      applyShortcut,
    ),
    shortcutFeedback,
    h("div", { class: "row" },
      h("label", { text: "Launch at startup" }),
      toggle(settings.autostart, (v) => { settings.autostart = v; void save(); }),
    ),
  );
}

// ── Boot ──────────────────────────────────────────────────────────────────────

async function main() {
  let readFailure = false;
  let boot: Awaited<ReturnType<typeof Bridge.boot>> = null;
  try {
    boot = await Bridge.boot();
  } catch (error) {
    readFailure = true;
    reportBridgeFailure(error, "settings startup");
  }
  if (boot) {
    settings = { ...settings, ...boot.settings };
    Motion.setReducedMotion(settings.reducedMotion);
    version = boot.version;
    monitors = boot.monitors;
    shortcutStatus = boot.shortcutStatus;
  }
  let status: HookStatus | null = null;
  try {
    status = await Bridge.hooksStatus();
  } catch (error) {
    readFailure = true;
    reportBridgeFailure(error, "hook status startup");
  }

  let codexStatus: CodexHookStatus | null = null;
  try {
    codexStatus = await Bridge.codexHooksStatus();
  } catch (error) {
    readFailure = true;
    reportBridgeFailure(error, "Codex hook status startup");
  }

  let hasKey: boolean | null = null;
  try {
    hasKey = await Bridge.secretPresent("anthropic-api-key");
  } catch (error) {
    readFailure = true;
    reportBridgeFailure(error, "chat credential startup status");
  }

  const keys = [
    "stripe-api-key", "github-token", "vercel-token",
    "n8n-url", "n8n-api-key", "resend-api-key", "notion-api-key", "calcom-api-key",
  ];
  const present: Record<string, boolean | null> = {};
  for (const k of keys) {
    try {
      present[k] = await Bridge.secretPresent(k);
    } catch (error) {
      readFailure = true;
      present[k] = null;
      reportBridgeFailure(error, "integration credential startup status");
    }
  }

  clear(root);
  root.append(
    h("h1", {}, h("span", { text: "Anti-Scrolling-Notch" }), h("span", { class: "version", text: version })),
    pageNotice,
    claudeSection(status),
    codexHooksSection(codexStatus),
    apiSection(hasKey),
    integrationsSection(present),
    generalSection(),
    h("div", {
      class: "hint",
      text: "No telemetry. Network requests only go to the services you configure yourself.",
    }),
  );
  if (readFailure) {
    showPageNotice("Some settings or credential status could not be loaded. Affected indicators are marked unavailable.", "warn");
  }

  void onEvent<Settings>("settings-changed", (s) => {
    settings = { ...settings, ...s };
    Motion.setReducedMotion(settings.reducedMotion);
  });
}

void main();
