// Every theme reads: each text colour on what it stands on, in the ready
// themes and in seeds made by chance. Run: `npm test` in app/.
import { test } from "node:test";
import assert from "node:assert/strict";
import { PRESETS, WHO, palette, parseSeed, randomSeed, ratio, seedText, type Seed } from "../src/lib/color.ts";

/// Text, what it stands on, the ratio it needs (in high contrast: 7).
const PAIRS: [string, string, number][] = [
  ["--fg", "--page", 4.5],
  ["--muted", "--page", 4.5],
  ["--heading", "--page", 4.5],
  ["--star", "--page", 4.5],
  ["--name-ref", "--page", 4.5],
  ["--warn", "--page", 4.5],
  ["--ok", "--page", 4.5],
  ["--danger", "--page", 4.5],
  ["--danger", "--chrome", 4.5],
  ["--fg", "--selection", 4.5],
  ["--chrome-fg", "--chrome", 4.5],
  ["--chrome-fg", "--chrome-2", 4.5],
  ["--chrome-fg", "--raised", 4.5],
  ["--chrome-muted", "--chrome", 4.5],
  ["--accent-ink", "--chrome", 4.5],
  ["--accent-ink", "--raised", 4.5],
  ...Array.from({ length: WHO }, (_, i) => [`--who-${i}`, "--page", 4.5] as [string, string, number]),
];

function reads(seed: Seed) {
  const p = palette(seed);
  const high = seed.contrast === "high";
  for (const [fg, bg, want] of PAIRS) {
    const r = ratio(p[fg], p[bg]);
    assert.ok(r >= (high && want >= 4.5 ? 7 : want) - 0.05, `${seedText(seed)}: ${fg} on ${bg} is ${r.toFixed(2)}`);
  }
  assert.ok(ratio("#fbf6ee", p["--claret"]) >= 4.45, `${seedText(seed)}: text on --claret`);
}

test("the ready themes read", () => {
  for (const [, , , seed] of PRESETS) reads(seed);
});

test("seeds made by chance read", () => {
  let last: Seed | null = null;
  for (let i = 0; i < 300; i++) {
    last = randomSeed(last);
    reads(last);
    reads({ ...last, contrast: "high" });
  }
});

test("dark themes read down to black, and a light one stops at white", () => {
  for (const [, , , seed] of PRESETS) {
    if (seed.mode !== "dark") continue;
    for (const depth of [-0.15, -0.08, 0.1]) for (const warmth of [0, 0.08]) reads({ ...seed, depth, warmth });
  }
  const black = palette({ ...PRESETS[0][3], mode: "dark", warmth: 0, depth: -0.15 });
  assert.equal(black["--desk"], "#000000");
  assert.equal(parseSeed("light 75 0.014 20 0.130 analogous normal -0.15")?.depth, 0);
  assert.equal(parseSeed("dark 75 0.014 20 0.130 analogous normal -0.15")?.depth, -0.15);
});

test("a seed comes back as it was kept", () => {
  for (const [, , , seed] of PRESETS) assert.deepEqual(parseSeed(seedText(seed)), seed);
  assert.equal(parseSeed("123 dark")?.mode, "dark");
  assert.equal(parseSeed("nonsense"), null);
});
