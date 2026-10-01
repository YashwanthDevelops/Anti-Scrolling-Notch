import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { readdirSync, readFileSync, statSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import test from "node:test";
import resources from "../resources.json" with { type: "json" };

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");

test("the retained development visuals and icons are replaceable resource references", () => {
  assert.equal(resources.profile, "coucou-mochi-development");
  assert.equal(resources.replaceable, true);
  for (const reference of [
    resources.assets.mochiRenderer,
    resources.assets.greetingRenderer,
    resources.assets.inlineIcons,
    resources.assets.applicationIcons,
  ]) {
    assert.ok(statSync(join(root, reference)).isDirectory() || statSync(join(root, reference)).isFile());
  }
});

test("all referenced sound files are bundled byte-for-byte from the inherited source", () => {
  const source = resolve(root, resources.assets.sounds.sourceDirectory);
  const output = join(root, "dist", resources.assets.sounds.publicDirectory.replace(/^\//, ""));
  const wavs = (directory) => readdirSync(directory).filter((file) => file.endsWith(".wav")).sort();
  const sourceFiles = wavs(source);
  const outputFiles = wavs(output);
  assert.equal(sourceFiles.length, 28, "the audited development sound set should remain intact");
  assert.deepEqual(outputFiles, sourceFiles);

  for (const file of sourceFiles) {
    const hash = (bytes) => createHash("sha256").update(bytes).digest("hex");
    assert.equal(hash(readFileSync(join(output, file))), hash(readFileSync(join(source, file))), file);
  }

  const player = readFileSync(join(root, "src", "core", "sound.ts"), "utf8");
  assert.ok(player.includes("resources.assets.sounds.publicDirectory"));
});
