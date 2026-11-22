// Dactile's colours, made from a few choices by colour theory rather than
// picked one by one.
//
// - OKLCH: lightness as the eye sees it, so a theme keeps its weight in any
//   hue, and a hue turned on the wheel stays as readable.
// - A theme is a seed: a base hue that tints the neutrals (how warm), an
//   accent hue (how vivid), a harmony that gives the second hue, light or
//   dark, normal or high contrast.
// - Roles, not colours: the neutrals carry most of the screen, the chrome
//   the rest, the accent only what acts, is picked or says "you are here".
//   Headings take the accent; characters the harmony's second hue; warnings
//   and what went well keep hues of their own (amber, green), in the
//   theme's lightness.
// - Every text role is pushed in lightness until it reads (WCAG 4.5:1, 7:1
//   in high contrast) on what it stands on; a colour out of the screen's
//   gamut loses chroma, never its hue.
// - Characters get hues a golden angle apart, all at one lightness and
//   chroma: none outweighs another, and a new one leaves the others be.

export type Mode = "light" | "dark";
export type Harmony = "mono" | "analogous" | "complementary" | "split" | "triadic";
export type Contrast = "normal" | "high";

export interface Seed {
  mode: Mode;
  /// The neutrals' hue, and their chroma: 0 grey, 0.06 clearly tinted.
  base: number;
  warmth: number;
  /// The accent's hue, and its chroma: 0.04 muted to 0.2 vivid.
  accent: number;
  vivid: number;
  harmony: Harmony;
  contrast: Contrast;
  /// How far the surfaces sit from white or black: 0 to 0.1; a dark
  /// theme goes below 0 too, down to black (`DEEPEST`).
  depth: number;
}

export const HARMONIES: [Harmony, string, string][] = [
  ["mono", "Tek renk", "One hue"],
  ["analogous", "Komşu", "Analogous"],
  ["complementary", "Tamamlayıcı", "Complementary"],
  ["split", "Bölünmüş tamamlayıcı", "Split complementary"],
  ["triadic", "Üçlü", "Triadic"],
];

/// How many character colours a theme has; the next one comes round again.
export const WHO = 12;
const GOLDEN = 137.508;

// ---------------------------------------------------------------- OKLCH

type Rgb = [number, number, number];

const lin = (c: number) => (c <= 0.04045 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4);
const gam = (c: number) => (c <= 0.0031308 ? 12.92 * c : 1.055 * c ** (1 / 2.4) - 0.055);

/// OKLCH to linear sRGB, unclamped.
function linear(l: number, c: number, h: number): Rgb {
  const a = c * Math.cos((h * Math.PI) / 180);
  const b = c * Math.sin((h * Math.PI) / 180);
  const l_ = (l + 0.3963377774 * a + 0.2158037573 * b) ** 3;
  const m_ = (l - 0.1055613458 * a - 0.0638541728 * b) ** 3;
  const s_ = (l - 0.0894841775 * a - 1.291485548 * b) ** 3;
  return [
    4.0767416621 * l_ - 3.3077115913 * m_ + 0.2309699292 * s_,
    -1.2684380046 * l_ + 2.6097574011 * m_ - 0.3413193965 * s_,
    -0.0041960863 * l_ - 0.7034186147 * m_ + 1.707614701 * s_,
  ];
}

const inGamut = (rgb: Rgb) => rgb.every((v) => v >= -1e-4 && v <= 1 + 1e-4);

/// A colour the screen can show: chroma lowered until it fits, the hue kept.
export interface Colour {
  l: number;
  c: number;
  h: number;
}
export function fit(l: number, c: number, h: number): Colour {
  l = Math.min(1, Math.max(0, l));
  if (inGamut(linear(l, c, h))) return { l, c, h };
  let lo = 0;
  let hi = c;
  for (let i = 0; i < 20; i++) {
    const mid = (lo + hi) / 2;
    if (inGamut(linear(l, mid, h))) lo = mid;
    else hi = mid;
  }
  return { l, c: lo, h };
}

function rgb(k: Colour): Rgb {
  return linear(k.l, k.c, k.h).map((v) => Math.min(1, Math.max(0, v))) as Rgb;
}

export function hex(k: Colour): string {
  return (
    "#" +
    rgb(k)
      .map((v) =>
        Math.round(gam(v) * 255)
          .toString(16)
          .padStart(2, "0"),
      )
      .join("")
  );
}

/// WCAG relative luminance, of a colour or of `#rrggbb`.
export function luminance(k: Colour | string): number {
  const [r, g, b] =
    typeof k === "string" ? ([1, 3, 5].map((i) => lin(parseInt(k.slice(i, i + 2), 16) / 255)) as Rgb) : rgb(k);
  return 0.2126 * r + 0.7152 * g + 0.0722 * b;
}

export function ratio(a: Colour | string, b: Colour | string): number {
  const [x, y] = [luminance(a), luminance(b)].sort((p, q) => q - p);
  return (x + 0.05) / (y + 0.05);
}

/// `k` made lighter or darker, away from `on`, until it reads at `want`:1.
export function reach(k: Colour, on: Colour | string, want: number): Colour {
  const away = luminance(on) > 0.18 ? -1 : 1;
  let c = fit(k.l, k.c, k.h);
  // A little over: the colour is rounded to #rrggbb after.
  for (let i = 0; i < 100 && ratio(c, on) < want + 0.08; i++) {
    const l = c.l + away * 0.01;
    if (l < 0 || l > 1) break;
    c = fit(l, k.c, k.h);
  }
  return c;
}

const turn = (h: number, by: number) => (((h + by) % 360) + 360) % 360;

/// The wheel's points of a harmony: the accent first.
export function harmony(seed: Seed): number[] {
  const a = seed.accent;
  switch (seed.harmony) {
    case "mono":
      return [a];
    case "analogous":
      return [a, turn(a, 35), turn(a, -35)];
    case "complementary":
      return [a, turn(a, 180)];
    case "split":
      return [a, turn(a, 150), turn(a, 210)];
    case "triadic":
      return [a, turn(a, 120), turn(a, 240)];
  }
}

// -------------------------------------------------------------- palette

/// The lowest depth a dark theme takes: its desk at black.
export const DEEPEST = -0.15;
/// The most a theme's neutrals are tinted.
export const MOST_WARMTH = 0.08;

/// A depth as a theme of this mode takes it.
export const depthIn = (mode: Mode, depth: number) => Math.min(0.1, Math.max(mode === "dark" ? DEEPEST : 0, depth));

/// Every colour variable of theme.css, made from a seed.
export function palette(seed: Seed): Record<string, string> {
  const dark = seed.mode === "dark";
  const high = seed.contrast === "high";
  const text = high ? 7 : 4.5;
  const hue = seed.base;
  // Neutrals: high contrast takes the tint down, so nothing competes.
  const w = high ? seed.warmth / 3 : seed.warmth;
  // Depth takes a light theme's surfaces down, a dark one's up.
  const d = (dark ? 1 : -1) * depthIn(seed.mode, seed.depth);
  const n = (l: number, c = w) => fit(l + d, c, hue);

  // Lightnesses, from the back to the front.
  const L = dark
    ? { desk: 0.15, page: 0.205, chrome: 0.195, chrome2: 0.235, line: 0.3, raised: 0.235, fg: high ? 0.97 : 0.86, cfg: high ? 0.97 : 0.87 }
    : { desk: 0.86, page: high ? 1 : 0.985, chrome: 0.955, chrome2: 0.915, line: 0.86, raised: 0.985, fg: high ? 0.1 : 0.23, cfg: high ? 0.1 : 0.25 };
  const desk = n(L.desk);
  const page = n(L.page, high ? 0 : w * 0.8);
  const chrome = n(L.chrome);
  const chrome2 = n(L.chrome2);
  const line = n(high ? (dark ? 0.45 : 0.6) : L.line, w * 1.2);
  const raised = n(L.raised);
  // What stands on the chrome reads on each of its surfaces.
  const onChrome = (k: Colour) => [chrome, chrome2, raised].reduce((c, bg) => reach(c, bg, text), k);
  const fg = reach(n(L.fg, w * 1.4), page, text);
  const cfg = onChrome(n(L.cfg, w * 1.4));
  const muted = reach(n(0.6, w * 1.5), page, text);
  const cmuted = onChrome(n(0.6, w * 1.5));
  const faint = n(dark ? 0.42 : 0.72, w * 1.2);

  // The accent: a fill that white text reads on, and as ink on the chrome
  // and the page.
  const [a, second] = [seed.accent, harmony(seed)[1] ?? turn(seed.accent, 25)];
  const v = seed.vivid;
  const white = "#fbf6ee";
  const claret = reach(fit(dark ? 0.5 : 0.52, v, a), white, high ? 7 : 4.5);
  const claretDeep = fit(claret.l - 0.07, v, a);
  const ink = onChrome(fit(dark ? 0.76 : 0.48, v, a));
  const heading = reach(fit(dark ? 0.74 : 0.45, v, a), page, text);
  const selection = fit(dark ? 0.33 : 0.88, Math.min(v, 0.06), a);

  // Characters: the second hue, less loud than the accent.
  const star = reach(fit(dark ? 0.8 : 0.4, v * 0.75, second), page, text);
  const nameRef = reach(fit(dark ? 0.78 : 0.42, v * 0.45, second), page, text);

  // What a colour means stays its hue; the theme gives the lightness.
  const warn = reach(fit(dark ? 0.78 : 0.52, 0.13, 60), page, text);
  const ok = reach(fit(dark ? 0.78 : 0.5, 0.11, 150), page, text);
  const danger = onChrome(reach(fit(dark ? 0.72 : 0.48, 0.16, 27), page, text));

  // One lightness for every character, the one that lets the hardest hue
  // read: in a light theme the lowest of them, in a dark one the highest.
  const whoC = Math.min(v, 0.13);
  const hues = Array.from({ length: WHO }, (_, i) => turn(second, i * GOLDEN));
  const ls = hues.map((h) => reach(fit(dark ? 0.78 : 0.5, whoC, h), page, text).l);
  const whoL = dark ? Math.max(...ls) : Math.min(...ls);
  const who = hues.map((h) => reach(fit(whoL, whoC, h), page, text));

  const out: Record<string, string> = {
    "--desk": hex(desk),
    "--bg": hex(desk),
    "--page": hex(page),
    "--fg": hex(fg),
    "--muted": hex(muted),
    "--faint": hex(faint),
    "--chrome": hex(chrome),
    "--chrome-2": hex(chrome2),
    "--chrome-line": hex(line),
    "--chrome-fg": hex(cfg),
    "--chrome-muted": hex(cmuted),
    "--raised": hex(raised),
    "--claret": hex(claret),
    "--claret-deep": hex(claretDeep),
    "--accent-ink": hex(ink),
    "--heading": hex(heading),
    "--link": hex(ink),
    "--star": hex(star),
    "--name-ref": hex(nameRef),
    "--warn": hex(warn),
    "--ok": hex(ok),
    "--danger": hex(danger),
    "--on-claret": white,
    "--selection": hex(selection),
    "color-scheme": seed.mode,
  };
  who.forEach((k, i) => (out[`--who-${i}`] = hex(k)));
  return out;
}

// ---------------------------------------------------------------- seeds

/// Name, Turkish, English, seed: the themes offered ready.
export const PRESETS: [string, string, string, Seed][] = [
  ["paper", "Kâğıt", "Paper", { mode: "light", base: 75, warmth: 0.014, accent: 20, vivid: 0.13, harmony: "analogous", contrast: "normal", depth: 0 }],
  ["sepia", "Sepya", "Sepia", { mode: "light", base: 78, warmth: 0.04, accent: 38, vivid: 0.11, harmony: "analogous", contrast: "normal", depth: 0.04 }],
  ["winter", "Kış", "Winter", { mode: "light", base: 245, warmth: 0.012, accent: 255, vivid: 0.12, harmony: "split", contrast: "normal", depth: 0.01 }],
  ["olive", "Zeytin", "Olive", { mode: "light", base: 110, warmth: 0.022, accent: 140, vivid: 0.1, harmony: "complementary", contrast: "normal", depth: 0.02 }],
  ["night", "Gece", "Night", { mode: "dark", base: 70, warmth: 0.003, accent: 18, vivid: 0.09, harmony: "analogous", contrast: "normal", depth: 0 }],
  ["dark", "Koyu", "Dark", { mode: "dark", base: 90, warmth: 0.009, accent: 22, vivid: 0.11, harmony: "analogous", contrast: "normal", depth: 0.05 }],
  ["lavender", "Lavanta", "Lavender", { mode: "dark", base: 295, warmth: 0.02, accent: 305, vivid: 0.11, harmony: "triadic", contrast: "normal", depth: 0.03 }],
  ["black", "Siyah", "Black", { mode: "dark", base: 0, warmth: 0, accent: 22, vivid: 0.12, harmony: "analogous", contrast: "normal", depth: -0.15 }],
  ["contrast", "Yüksek kontrast", "High contrast", { mode: "light", base: 0, warmth: 0, accent: 25, vivid: 0.18, harmony: "mono", contrast: "high", depth: 0 }],
];

/// The accents of older settings, as hues.
const ACCENT_HUES: Record<string, number> = { claret: 20, blue: 255, green: 155, plum: 315, ochre: 70, clay: 40 };

export function preset(name: string): Seed | undefined {
  return PRESETS.find(([n]) => n === name)?.[3];
}

/// A seed as the settings keep it: `light 75 0.014 20 0.13 analogous normal 0`.
export function seedText(s: Seed): string {
  return [s.mode, Math.round(s.base), s.warmth.toFixed(3), Math.round(s.accent), s.vivid.toFixed(3), s.harmony, s.contrast, s.depth.toFixed(2)].join(" ");
}

const HARMONY_NAMES = HARMONIES.map(([h]) => h);

/// A seed from the settings; an older surprise (`123 dark`) too.
export function parseSeed(text: string): Seed | null {
  const p = text.trim().split(/\s+/);
  if (p.length === 2) {
    const h = parseInt(p[0], 10);
    if (Number.isNaN(h)) return null;
    return { mode: p[1] === "light" ? "light" : "dark", base: h, warmth: 0.03, accent: turn(h, 150), vivid: 0.12, harmony: "complementary", contrast: "normal", depth: 0 };
  }
  if (p.length !== 8) return null;
  const [mode, base, warmth, accent, vivid, harm, contrast, depth] = p;
  const num = [base, warmth, accent, vivid, depth].map(Number);
  if (num.some(Number.isNaN) || !HARMONY_NAMES.includes(harm as Harmony)) return null;
  const m: Mode = mode === "dark" ? "dark" : "light";
  return {
    mode: m,
    base: turn(num[0], 0),
    warmth: Math.min(MOST_WARMTH, Math.max(0, num[1])),
    accent: turn(num[2], 0),
    vivid: Math.min(0.22, Math.max(0.03, num[3])),
    harmony: harm as Harmony,
    contrast: contrast === "high" ? "high" : "normal",
    depth: depthIn(m, num[4]),
  };
}

/// The seed the settings wear: a preset (with an older accent on it), or
/// the user's own.
export function seedOf(theme: string, custom: string, accent: string): Seed {
  if (theme === "surprise") return parseSeed(custom) ?? PRESETS[0][3];
  const p = preset(theme) ?? PRESETS[0][3];
  const hue = ACCENT_HUES[accent];
  return hue === undefined || accent === "claret" ? p : { ...p, accent: hue };
}

/// A new seed by chance, in a harmony, never close to the last one.
export function randomSeed(last: Seed | null): Seed {
  let base = Math.floor(Math.random() * 360);
  if (last && Math.abs(base - last.base) < 40) base = turn(base, 120);
  const harmony = HARMONY_NAMES[Math.floor(Math.random() * HARMONY_NAMES.length)];
  const accent = harmony === "mono" ? base : turn(base, [0, 30, 150, 180, 210][Math.floor(Math.random() * 5)]);
  return {
    mode: Math.random() < 0.5 ? "dark" : "light",
    base,
    warmth: 0.01 + Math.random() * 0.04,
    accent,
    vivid: 0.08 + Math.random() * 0.1,
    harmony,
    contrast: "normal",
    depth: Math.random() * 0.06,
  };
}
