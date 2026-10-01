import assert from "node:assert/strict";
import { after, before, test } from "node:test";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { createServer } from "vite";

const originalDocument = globalThis.document;
const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
let vite;
let Motion;
let BotEngine;
let Ease;
let Greeting;

before(async () => {
  vite = await createServer({
    root,
    configFile: false,
    server: { middlewareMode: true, watch: null },
    appType: "custom",
    optimizeDeps: { noDiscovery: true, include: [], holdUntilCrawlEnd: false },
  });
  ({ Motion } = await vite.ssrLoadModule("/src/core/motion.ts"));
  ({ BotEngine } = await vite.ssrLoadModule("/src/mochi/engine.ts"));
  ({ Ease } = await vite.ssrLoadModule("/src/core/anim.ts"));
  ({ Greeting } = await vite.ssrLoadModule("/src/mochi/greeting.ts"));
});

after(() => {
  Motion?.setReducedMotion(false);
  globalThis.document = originalDocument;
  return vite?.close();
});

test("reduced motion preference updates the shared document treatment", () => {
  assert.equal(Motion.reducedMotion, false);
  const classes = new Set();
  globalThis.document = {
    documentElement: {
      classList: {
        toggle(name, enabled) {
          if (enabled) classes.add(name);
          else classes.delete(name);
        },
      },
    },
  };

  Motion.setReducedMotion(true);
  assert.equal(Motion.reducedMotion, true);
  assert.equal(classes.has("reduce-motion"), true);

  Motion.setReducedMotion(false);
  assert.equal(Motion.reducedMotion, false);
  assert.equal(classes.has("reduce-motion"), false);
});

test("reduced motion settles Mochi and suppresses decorative particles", () => {
  const bot = new BotEngine();
  bot.anim("sx", [[1.2, 200, Ease.out]]);
  bot.emit("spark", 1);
  assert.equal(bot.tweens.size, 1);
  assert.equal(bot.particles.length, 1);

  bot.setReducedMotion(true);
  bot.setState("error", true);
  bot.emit("spark", 3);
  bot.update(1 / 60);

  assert.equal(bot.state, "error");
  assert.equal(bot.tweens.size, 0);
  assert.equal(bot.particles.length, 0);
  assert.equal(bot.sx, 1);
  assert.equal(bot.busy, false);

  bot.setReducedMotion(false);
  bot.blink();
  assert.equal(bot.tweens.size, 1, "normal tween behavior resumes when reduced motion is disabled");
});

test("reduced motion completes the greeting without its timed choreography", () => {
  Motion.setReducedMotion(true);
  const greeting = new Greeting();
  let completions = 0;
  greeting.onComplete = () => completions++;

  greeting.start();

  assert.equal(greeting.done, true);
  assert.equal(completions, 1);
});
