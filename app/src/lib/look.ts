// How the app and its page look: the settings as theme.css's variables,
// and what the settings offer.
import type { Look } from "./api";

/// Name, Turkish, English, page, ink: as theme.css has them.
export const THEMES: [string, string, string, string, string][] = [
  ["paper", "Kâğıt", "Paper", "#fbf8f1", "#221f1b"],
  ["sepia", "Sepya", "Sepia", "#f6ead2", "#3a2a1a"],
  ["night", "Gece", "Night", "#1f2530", "#d6dbe4"],
  ["contrast", "Yüksek kontrast", "High contrast", "#ffffff", "#000000"],
  ["dark", "Koyu", "Dark", "#151515", "#e6e6e6"],
];

/// Name, Turkish, English, on a light page, on a dark one.
export const ACCENTS: [string, string, string, string, string][] = [
  ["claret", "Bordo", "Claret", "#b0444a", "#e08f94"],
  ["blue", "Mavi", "Blue", "#2f5f9e", "#8ab4e8"],
  ["green", "Yeşil", "Green", "#2f6f4f", "#7cc4a0"],
  ["plum", "Erik", "Plum", "#7a3f8a", "#c79ad6"],
  ["ochre", "Toprak", "Ochre", "#8a5a10", "#d9a95a"],
];

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
  const { theme, accent, page_font, page_size, line_spacing, page_width, page_zoom, animations } = s;
  return { theme, accent, page_font, page_size, line_spacing, page_width, page_zoom, animations };
}

export function applyLook(s: Look) {
  const root = document.documentElement;
  root.dataset.theme = s.theme || "paper";
  root.dataset.accent = s.accent || "claret";
  root.dataset.animations = s.animations ? "on" : "off";
  root.style.setProperty("--page-font", s.page_font ? `"${s.page_font}", var(--serif)` : "var(--serif)");
  root.style.setProperty("--page-size", `${within(s.page_size, 12, 28)}px`);
  root.style.setProperty("--line-spacing", String(within(s.line_spacing, 120, 220) / 100));
  root.style.setProperty("--page-width", String(within(s.page_width, 32, 64)));
}

/// The limits are `Config::normal`'s in core.
const within = (n: number, least: number, most: number) => Math.min(most, Math.max(least, n || least));
