import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const identity = JSON.parse(readFileSync(join(root, "runtime-identity.json"), "utf8"));
const tauri = JSON.parse(readFileSync(join(root, "src-tauri", "tauri.conf.json"), "utf8"));
const frontend = JSON.parse(readFileSync(join(root, "package.json"), "utf8"));
const workspace = readFileSync(join(root, "Cargo.toml"), "utf8");
const identityCargo = readFileSync(join(root, "runtime-identity", "Cargo.toml"), "utf8");
const appCargo = readFileSync(join(root, "src-tauri", "Cargo.toml"), "utf8");
const hookCargo = readFileSync(join(root, "hook", "Cargo.toml"), "utf8");

function cargoValue(source, section, key) {
  const lines = source.split(/\r?\n/);
  const start = lines.findIndex((line) =>
    line.trim() === `[${section}]` || line.trim() === `[[${section}]]`,
  );
  assert.notEqual(start, -1, `missing Cargo section [${section}]`);
  let end = start + 1;
  while (end < lines.length && !/^\[\[?[^\]]+\]\]?\s*$/.test(lines[end].trim())) end++;
  const body = lines.slice(start, end).join("\n");
  const value = body.match(new RegExp(`^${key}\\s*=\\s*"([^"]+)"`, "m"));
  assert.ok(value, `missing ${key} in Cargo [${section}]`);
  return value[1];
}

assert.equal(tauri.productName, identity.productName);
assert.equal(tauri.identifier, identity.appIdentifier);
assert.equal(cargoValue(identityCargo, "package", "name"), identity.identityCargoPackage);
assert.equal(cargoValue(appCargo, "package", "name"), identity.appCargoPackage);
assert.equal(cargoValue(appCargo, "lib", "name"), identity.appLibrary);
assert.equal(cargoValue(hookCargo, "package", "name"), identity.relayPackage);
assert.equal(cargoValue(hookCargo, "bin", "name"), identity.relayPackage);
assert.equal(frontend.name, identity.frontendPackage);
assert.ok(workspace.includes('"runtime-identity"'), "runtime identity crate must be in the workspace");
assert.ok(appCargo.includes('anti-scrolling-notch-runtime-identity = { path = "../runtime-identity" }'));
assert.ok(hookCargo.includes('anti-scrolling-notch-runtime-identity = { path = "../runtime-identity" }'));
assert.equal(
  tauri.bundle.resources[`../target/release/${identity.relayExecutable}`],
  identity.relayExecutable,
);

const appMain = readFileSync(join(root, "src-tauri", "src", "main.rs"), "utf8");
const appLib = readFileSync(join(root, "src-tauri", "src", "lib.rs"), "utf8");
const hookMain = readFileSync(join(root, "hook", "src", "main.rs"), "utf8");
const appPipe = readFileSync(join(root, "src-tauri", "src", "pipe.rs"), "utf8");
const codexPipe = readFileSync(join(root, "src-tauri", "src", "codex_pipe.rs"), "utf8");
const settings = readFileSync(join(root, "src-tauri", "src", "settings.rs"), "utf8");
const secrets = readFileSync(join(root, "src-tauri", "src", "secrets.rs"), "utf8");
const installerHooks = readFileSync(join(root, "src-tauri", "nsis", "hooks.nsh"), "utf8");
const releaseWorkflow = readFileSync(join(root, "..", ".github", "workflows", "windows.yml"), "utf8");

assert.ok(appMain.includes(`${identity.appLibrary}::run()`));
assert.ok(appLib.includes(".app_name(identity::AUTOSTART_VALUE_NAME)"));
assert.ok(hookMain.includes("APP_PIPE_PREFIX"));
assert.ok(hookMain.includes("CODEX_PIPE_PREFIX"));
assert.ok(appPipe.includes("identity::APP_PIPE_PREFIX"));
assert.ok(codexPipe.includes("identity::CODEX_PIPE_PREFIX"));
assert.ok(settings.includes("identity::STORAGE_DIRECTORY"));
assert.ok(settings.includes("identity::RELAY_EXECUTABLE"));
assert.ok(secrets.includes("identity::CREDENTIAL_NAMESPACE"));
assert.ok(installerHooks.includes(`$LOCALAPPDATA\\${identity.storageDirectory}\\bin`));
assert.ok(!installerHooks.includes("$LOCALAPPDATA\\Coucou"), "the new uninstaller must preserve Coucou data");
assert.ok(releaseWorkflow.includes("REL11_WINDOWS_ASSET_CLEARANCE"));
assert.ok(releaseWorkflow.includes("-cne 'cleared'"), "distribution must stay gated on explicit REL-11 clearance");
assert.ok(releaseWorkflow.includes(`${identity.productName}-Windows-setup`));
assert.ok(frontend.scripts.predev.includes(identity.relayPackage));
assert.ok(frontend.scripts.prebuild.includes(identity.relayPackage));

console.log(`PASS runtime identity: ${identity.productName} (${identity.appIdentifier})`);
