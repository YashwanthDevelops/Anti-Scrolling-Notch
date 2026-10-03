import assert from "node:assert/strict";
import { readFile, readdir } from "node:fs/promises";
import { dirname, relative, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import test from "node:test";
import ts from "typescript";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const srcRoot = resolve(root, "src");

async function typescriptFiles(directory = srcRoot) {
  const entries = await readdir(directory, { withFileTypes: true });
  const nested = await Promise.all(entries.map(async (entry) => {
    const path = resolve(directory, entry.name);
    if (entry.isDirectory()) return typescriptFiles(path);
    return entry.isFile() && /\.(?:ts|js)$/.test(path) ? [path] : [];
  }));
  return nested.flat();
}

function expressionPath(expression) {
  if (ts.isIdentifier(expression)) return expression.text;
  if (ts.isPropertyAccessExpression(expression)) {
    const parent = expressionPath(expression.expression);
    return parent ? `${parent}.${expression.name.text}` : null;
  }
  if (ts.isElementAccessExpression(expression)) {
    const parent = expressionPath(expression.expression);
    const argument = expression.argumentExpression;
    const key = argument && (ts.isStringLiteral(argument) || ts.isNumericLiteral(argument))
      ? argument.text
      : null;
    return parent && key !== null ? `${parent}.${key}` : null;
  }
  if (ts.isParenthesizedExpression(expression)) return expressionPath(expression.expression);
  return null;
}

function assignmentOperator(kind) {
  return kind >= ts.SyntaxKind.FirstAssignment && kind <= ts.SyntaxKind.LastAssignment;
}

async function frontendSources() {
  return Promise.all((await typescriptFiles()).map(async (path) => {
    const text = await readFile(path, "utf8");
    return {
      path,
      name: relative(srcRoot, path).replaceAll("\\", "/"),
      text,
      ast: ts.createSourceFile(path, text, ts.ScriptTarget.Latest, true, ts.ScriptKind.TS),
    };
  }));
}

test("frontend external transport stays behind the bridge and local sound loader", async () => {
  const sources = await frontendSources();
  const tauriImports = [];
  const networkCalls = [];
  const processModules = [];
  const forbiddenClients = [];
  const directBrokerSyncCalls = [];

  for (const source of sources) {
    function visit(node) {
      if (ts.isImportDeclaration(node) && ts.isStringLiteral(node.moduleSpecifier)) {
        const module = node.moduleSpecifier.text;
        if (module.startsWith("@tauri-apps/") && source.name !== "core/bridge.ts") {
          tauriImports.push(`${source.name}: ${module}`);
        }
        if (/^(node:)?(child_process|node:child_process)$/.test(module)) {
          processModules.push(`${source.name}: ${module}`);
        }
        if (/^(?:axios|octokit|@octokit\/|got|ky)(?:\/|$)/i.test(module)) {
          forbiddenClients.push(`${source.name}: ${module}`);
        }
      }

      if (ts.isCallExpression(node)) {
        const callee = expressionPath(node.expression);
        if (callee === "invoke" && source.name !== "core/bridge.ts") {
          tauriImports.push(`${source.name}: direct invoke()`);
        }
        if (callee === "Bridge.syncBroker" && source.name !== "core/view-store.ts") {
          directBrokerSyncCalls.push(source.name);
        }
        if (/(^|\.)fetch$/.test(callee ?? "")
          || /(^|\.)(axios|XMLHttpRequest|WebSocket|EventSource)$/.test(callee ?? "")) {
          networkCalls.push(`${source.name}: ${callee}()`);
        }
      }
      if (ts.isNewExpression(node)) {
        const callee = expressionPath(node.expression);
        if (/(^|\.)(XMLHttpRequest|WebSocket|EventSource)$/.test(callee ?? "")) {
          networkCalls.push(`${source.name}: new ${callee}()`);
        }
      }

      ts.forEachChild(node, visit);
    }
    visit(source.ast);
  }

  assert.deepEqual(tauriImports, [], "Tauri command/event access must use core/bridge.ts");
  assert.deepEqual(directBrokerSyncCalls, [], "broker synchronization must use the view store");
  assert.deepEqual(processModules, [], "the WebView must not start external processes");
  assert.deepEqual(forbiddenClients, [], "the WebView must not own service API clients");
  assert.deepEqual(
    networkCalls.filter((call) => call !== "core/sound.ts: fetch()"),
    [],
    "the only current fetch is the local inherited sound-asset loader",
  );
  assert.ok(networkCalls.includes("core/sound.ts: fetch()"));
});

test("broker authority stays in the read-only view store", async () => {
  const sources = await frontendSources();
  const writes = [];
  const mutations = [];
  const leakedInternals = [];
  const appState = sources.find((source) => source.name === "core/state.ts");
  const viewStore = sources.find((source) => source.name === "core/view-store.ts");
  assert.ok(appState);
  assert.ok(viewStore);

  for (const source of sources) {
    function visit(node) {
      if (ts.isBinaryExpression(node) && assignmentOperator(node.operatorToken.kind)) {
        const target = expressionPath(node.left);
        if (target?.startsWith("BrokerView.snapshot")) writes.push(`${source.name}: ${target}`);
      }
      if (ts.isCallExpression(node)) {
        const target = expressionPath(node.expression);
        if (target?.startsWith("BrokerView.snapshot.")
          && /\.(push|pop|shift|unshift|splice|sort|reverse|copyWithin|fill)$/.test(target)) {
          mutations.push(`${source.name}: ${target}`);
        }
      }
      if (source.name !== "core/view-store.ts" && ts.isIdentifier(node)
        && (node.text === "snapshotValue" || node.text === "applyUpdate")) {
        leakedInternals.push(`${source.name}: ${node.text}`);
      }
      ts.forEachChild(node, visit);
    }
    visit(source.ast);
  }

  const authoritativeCollections = new Set([
    "sessions", "turns", "toolItems", "pendingRequests", "repositories",
  ]);
  const legacyAuthorityFields = [];
  function inspectAppState(node) {
    if (ts.isClassDeclaration(node) && node.name?.text === "AppState") {
      for (const member of node.members) {
        if (!ts.isPropertyDeclaration(member)) continue;
        const name = member.name && ts.isIdentifier(member.name) ? member.name.text : null;
        if (name && authoritativeCollections.has(name)) legacyAuthorityFields.push(name);
      }
    }
    ts.forEachChild(node, inspectAppState);
  }
  inspectAppState(appState.ast);

  assert.deepEqual(writes, [], "components cannot assign through the broker snapshot");
  assert.deepEqual(mutations, [], "components cannot mutate arrays in the broker snapshot");
  assert.deepEqual(leakedInternals, [], "snapshot reduction internals stay private to the store");
  assert.deepEqual(legacyAuthorityFields, [], "AppState cannot duplicate broker-owned collections");
  assert.match(viewStore.text, /private snapshotValue/);
  assert.match(viewStore.text, /get snapshot\(\): DeepReadonly<BrokerSnapshot>/);
});
