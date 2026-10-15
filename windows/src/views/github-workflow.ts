import { Bridge, handleBridgeCall, onEvent, reportBridgeFailure, safeBridgeFailure, type GithubSnapshot } from "../core/bridge";
import { BrokerView } from "../core/view-store";
import { acceptsWorkflowSnapshot, projectWorkflowHealth } from "../core/github-model";
import { h, clear, dot } from "./dom";

/** Workflow card with an explicit repository path and per-click gh opt-in. */
export function buildGithubWorkflowCard() {
  const path = h("input", {
    class: "github-path",
    type: "text",
    autocomplete: "off",
    spellcheck: "false",
    placeholder: "Local repository path",
    "aria-label": "Local repository path",
  }) as HTMLInputElement;
  const connect = h("button", { class: "link-btn github-connect", text: "Connect with gh" });
  const disconnect = h("button", { class: "link-btn github-disconnect", text: "Disconnect" });
  const status = h("div", { class: "github-health" }, dot("#A1A1AA", 6), h("span", { text: "Not connected" }));
  const repository = h("div", { class: "github-repository" });
  const pr = h("div", { class: "github-pr" });
  const checks = h("div", { class: "github-checks" });
  const feedback = h("div", { class: "github-feedback", role: "status" });
  const el = h("section", { class: "github-workflow" },
    h("div", { class: "github-head" },
      h("div", { class: "monitor-section-title", text: "GitHub workflow" }),
      status,
    ),
    h("div", { class: "github-controls" }, path, connect),
    h("div", { class: "github-optin", text: "Connecting uses your existing gh login and enables background refresh about every five minutes. The selected path is sent only after you click Connect." }),
    disconnect,
    repository,
    pr,
    checks,
    feedback,
  );

  let snapshot: GithubSnapshot | null = null;
  let busy = false;
  let connected = false;
  let connectedWorktreeId: string | null = null;
  let connectedPath: string | null = null;
  let message = "";
  connect.addEventListener("click", () => void refresh(true));
  disconnect.addEventListener("click", () => void refresh(false));
  BrokerView.subscribe(render);
  void onEvent<GithubSnapshot>("github-snapshot", (update) => {
    if (acceptsWorkflowSnapshot(update, connected, connectedWorktreeId)) {
      snapshot = update;
      render();
    }
  }).catch((error: unknown) => reportBridgeFailure(error, "GitHub refresh event subscription"));

  async function refresh(useGh: boolean) {
    const requestedPath = path.value.trim();
    const repoPath = useGh ? requestedPath : connectedPath ?? snapshot?.repository.canonicalPath ?? requestedPath;
    if (useGh && !requestedPath) {
      message = "Enter the local repository path first.";
      render();
      path.focus();
      return;
    }
    if (busy) return;
    busy = true;
    message = useGh ? connected ? "Refreshing GitHub data…" : "Connecting with gh…" : "Disconnecting GitHub…";
    render();
    try {
      if (useGh && connected && connectedPath && !sameLocalPath(repoPath, connectedPath)) {
        await Bridge.githubSnapshot(connectedPath, false, false);
        connected = false;
        connectedWorktreeId = null;
        connectedPath = null;
        snapshot = null;
      }
      snapshot = await Bridge.githubSnapshot(repoPath, useGh, useGh && connected);
      connected = useGh && (snapshot.health === "ready" || (connected && !["notConnected", "ghUnavailable", "unauthenticated"].includes(snapshot.health)));
      connectedWorktreeId = connected ? snapshot.repository.worktreeId ?? null : null;
      connectedPath = connected ? snapshot.repository.canonicalPath : null;
      message = snapshot.error ?? "";
    } catch (error) {
      message = safeBridgeFailure(error, "GitHub connection failed.");
    } finally {
      busy = false;
      render();
    }
  }

  function render() {
    connect.textContent = busy ? "Working…" : connected ? "Refresh" : "Connect with gh";
    connect.toggleAttribute("disabled", busy);
    disconnect.toggleAttribute("disabled", busy);
    clear(status);
    const integration = BrokerView.snapshot.integrations.find((item) => item.identity === "integration_github_workflow");
    const health = projectWorkflowHealth(connected, integration?.configuration, integration?.connection, snapshot);
    status.append(dot(health.color, 6), h("span", { text: health.label }));
    disconnect.style.display = connected ? "" : "none";
    clear(repository);
    clear(pr);
    clear(checks);
    if (snapshot) {
      const identity = BrokerView.snapshot.repositories.find((item) => item.worktreeId === connectedWorktreeId);
      const owner = typeof identity?.owner === "string" ? identity.owner : snapshot.repository.owner;
      const name = typeof identity?.name === "string" ? identity.name : snapshot.repository.name;
      const branch = typeof identity?.branch === "string" ? identity.branch : snapshot.repository.branch;
      const headSha = typeof identity?.headSha === "string" ? identity.headSha : snapshot.repository.headSha;
      repository.append(h("div", { class: "github-repo-name", text: [owner, name].filter(Boolean).join("/") || "Selected repository" }));
      const meta = [branch, headSha ? headSha.slice(0, 8) : null].filter(Boolean).join(" · ");
      if (meta) repository.append(h("div", { class: "github-repo-meta", text: meta }));
      if (snapshot.stale) repository.append(h("div", { class: "github-repo-meta", text: "Showing cached data" }));

      if (snapshot.pullRequest) {
        const item = snapshot.pullRequest;
        const state = item.isDraft ? "Draft" : item.state;
        const sha = item.headSha ? `${item.headSha.slice(0, 8)} · ` : "";
        pr.append(h("div", { class: "github-pr-title", text: `#${item.number} ${item.title}` }));
        pr.append(h("div", { class: "github-repo-meta", text: `${state} · ${sha}${item.headMatches ? "current head" : "different head"}` }));
        if (item.url) pr.append(h("button", { class: "link-btn github-open", text: "Open pull request", onclick: () => openSafeUrl(item.url) }));
      } else if (snapshot.health === "ready") {
        pr.append(h("div", { class: "github-repo-meta", text: "No pull request was returned for this repository and branch." }));
      }

      for (const check of snapshot.checks.slice(0, 5)) {
        const result = check.conclusion ? `${check.status} · ${check.conclusion}` : check.status;
        const row = h(check.url ? "button" : "div", {
          class: "github-check",
          "aria-label": `${check.name}: ${result}${check.url ? ", open check details" : ""}`,
          disabled: !check.url,
        }, dot(checkColor(check.status, check.conclusion, health.checksAreCurrent), 5),
          h("span", { class: "github-check-name", text: check.name }),
          h("span", { class: "github-check-state", text: result }),
        );
        if (check.url) row.addEventListener("click", () => openSafeUrl(check.url!));
        checks.append(row);
      }
    }
    clear(feedback);
    if (message) feedback.textContent = message;
  }

  function openSafeUrl(value: string) {
    let url: URL;
    try { url = new URL(value); } catch { return; }
    if (url.protocol === "https:") void handleBridgeCall(Bridge.openUrl(url.toString()), "open GitHub link");
  }

  render();
  return el;
}

function sameLocalPath(left: string, right: string): boolean {
  return left.replaceAll("/", "\\").replace(/[\\]+$/, "").toLocaleLowerCase()
    === right.replaceAll("/", "\\").replace(/[\\]+$/, "").toLocaleLowerCase();
}

function checkColor(status: string, conclusion: string | null, current: boolean): string {
  if (!current) return "#F5A524";
  const state = `${status} ${conclusion ?? ""}`.toLowerCase();
  if (/success|passed|neutral|skipped/.test(state)) return "#22C55E";
  if (/failure|failed|timed_out|cancelled|action_required/.test(state)) return "#F4505E";
  if (/pending|queued|in_progress|requested/.test(state)) return "#F5A524";
  return "#A1A1AA";
}
