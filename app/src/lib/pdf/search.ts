// Finding words in a PDF: each page's text as one string, and where in the
// page a piece of it lies.

import type { PDFDocumentProxy } from "pdfjs-dist";

export interface Piece {
  /// Where it starts in the page's text.
  at: number;
  length: number;
  x: number;
  y: number;
  w: number;
  h: number;
}

export interface PageText {
  text: string;
  pieces: Piece[];
}

export interface Hit {
  page: number;
  from: number;
  to: number;
}

/// Same length as `s`, so what is found in it is found in `s`. Turkish has
/// its own dotted and dotless i.
export function fold(s: string, tr: boolean): string {
  let out = "";
  for (const c of s.split("")) {
    let low = tr && c === "I" ? "ı" : tr && c === "İ" ? "i" : c.toLowerCase();
    if (low.length !== 1) low = c;
    out += low;
  }
  return out;
}

const wordy = (c: string | undefined) => !!c && /[\p{L}\p{N}_]/u.test(c);

export interface Asked {
  exact: boolean;
  words: boolean;
  tr: boolean;
}

/// Where `query` is in `text`, as [from, to].
export function findIn(text: string, query: string, a: Asked): [number, number][] {
  if (!query) return [];
  const hay = a.exact ? text : fold(text, a.tr);
  const needle = a.exact ? query : fold(query, a.tr);
  const out: [number, number][] = [];
  for (let i = hay.indexOf(needle); i >= 0; i = hay.indexOf(needle, i + needle.length)) {
    const to = i + needle.length;
    if (!a.words || (!wordy(text[i - 1]) && !wordy(text[to]))) out.push([i, to]);
  }
  return out;
}

/// The hit to go to from `index`, or -1: past the end it comes round only
/// with `wrap`.
export function stepHit(index: number, count: number, back: boolean, wrap: boolean): number {
  if (!count) return -1;
  const next = index + (back ? -1 : 1);
  if (next >= 0 && next < count) return next;
  if (!wrap) return -1;
  return back ? count - 1 : 0;
}

/// Lines end in a space, so a phrase is found across them.
export function joined(items: { str: string; eol: boolean; x: number; y: number; w: number; h: number }[]): PageText {
  let text = "";
  const pieces: Piece[] = [];
  for (const i of items) {
    if (i.str) pieces.push({ at: text.length, length: i.str.length, x: i.x, y: i.y, w: i.w, h: i.h });
    text += i.str;
    if (i.eol && !text.endsWith(" ")) text += " ";
  }
  return { text, pieces };
}

/// The rectangles of a stretch of text, in PDF points. A letter's share of
/// its piece is taken as even: near enough for a highlight.
export function rectsOf(t: PageText, from: number, to: number): number[][] {
  const out: number[][] = [];
  for (const p of t.pieces) {
    const a = Math.max(from, p.at);
    const b = Math.min(to, p.at + p.length);
    if (a >= b) continue;
    const per = p.w / p.length;
    out.push([p.x + (a - p.at) * per, p.y - p.h * 0.2, p.x + (b - p.at) * per, p.y + p.h * 0.85]);
  }
  return out;
}

/// The pages' text, read once for each document.
export class Finder {
  private texts = new Map<number, PageText>();
  private doc: PDFDocumentProxy | null = null;

  async text(doc: PDFDocumentProxy, n: number): Promise<PageText> {
    if (doc !== this.doc) {
      this.doc = doc;
      this.texts.clear();
    }
    const known = this.texts.get(n);
    if (known) return known;
    const page = await doc.getPage(n);
    const content = await page.getTextContent();
    const items = content.items.flatMap((i) =>
      "str" in i ? [{ str: i.str, eol: i.hasEOL, x: i.transform[4], y: i.transform[5], w: i.width, h: i.height }] : [],
    );
    const t = joined(items);
    this.texts.set(n, t);
    return t;
  }

  known(n: number) {
    return this.texts.get(n);
  }
}
