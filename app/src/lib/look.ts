// How the app and its page look: the settings as theme.css's variables,
// and what the settings offer.
import type { Look } from "./api";
import { palette, seedOf, type Seed } from "./color";

/// The seed the settings wear.
export const lookSeed = (s: Look): Seed => seedOf(s.theme, s.surprise, s.accent);

/// Fonts a desktop is likely to have; `installedFonts` keeps those it does.
const FONTS = [
  "EB Garamond",
  "Libertinus Serif",
  "Noto Serif",
  "Liberation Serif",
  "DejaVu Serif",
  "Georgia",
  "Times New Roman",
  "Noto Sans",
  "Liberation Sans",
  "DejaVu Sans",
  "Courier Prime",
  "Liberation Mono",
  "Noto Sans Mono",
  "DejaVu Sans Mono",
];

/// Whether a font is there: text in it, with a fallback behind, comes out
/// wider or narrower than in the fallback alone.
function installed(font: string): boolean {
  const c = document.createElement("canvas").getContext("2d");
  if (!c) return true;
  const width = (f: string) => {
    c.font = `72px ${f}`;
    return c.measureText("mmmwwiiİığş Oyun").width;
  };
  return ["monospace", "serif"].some((g) => width(`"${font}", ${g}`) !== width(g));
}

/// The fonts to offer. One chosen earlier stays on the list, gone or not:
/// the page falls back to the app's own.
export function installedFonts(chosen: string): string[] {
  const here = FONTS.filter(installed);
  return chosen && !here.includes(chosen) ? [chosen, ...here] : here;
}

export function pickLook(s: Look): Look {
  const { theme, surprise, saved_surprises, accent, page_font, page_size, line_spacing, page_width, page_zoom, animations, character_colors } = s;
  return { theme, surprise, saved_surprises, accent, page_font, page_size, line_spacing, page_width, page_zoom, animations, character_colors };
}

export function applyLook(s: Look) {
  const root = document.documentElement;
  const seed = lookSeed(s);
  root.dataset.theme = s.theme || "paper";
  root.dataset.mode = seed.mode;
  root.dataset.animations = s.animations ? "on" : "off";
  root.dataset.who = s.character_colors === false ? "off" : "on";
  for (const [key, value] of Object.entries(palette(seed))) root.style.setProperty(key, value);
  root.style.setProperty("--page-font", s.page_font ? `"${s.page_font}", var(--serif)` : "var(--serif)");
  root.style.setProperty("--page-size", `${within(s.page_size, 12, 28)}px`);
  root.style.setProperty("--line-spacing", String(within(s.line_spacing, 120, 220) / 100));
  root.style.setProperty("--page-width", String(within(s.page_width, 32, 64)));
}

/// The limits are `Config::normal`'s in core.
const within = (n: number, least: number, most: number) => Math.min(most, Math.max(least, n || least));
