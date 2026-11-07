// Ctrl+F: a small box in the corner, as a word processor has, instead of the
// editor's own panel. Gelişmiş opens the rest.

import { SearchQuery } from "@codemirror/search";
import type { EditorView, Panel } from "@codemirror/view";
import { mount, unmount } from "svelte";
import FindBar from "./FindBar.svelte";

/// The interface's language, set by the writer.
export const findLang = { tr: true };

export interface Hit {
  from: number;
  to: number;
  line: number;
  before: string;
  match: string;
  after: string;
}

/// The matches with their lines, for the advanced window's list.
export function hitsOf(view: EditorView, q: SearchQuery, most = 200): Hit[] {
  if (!q.valid) return [];
  const cursor = q.getCursor(view.state) as Iterator<{ from: number; to: number }>;
  const hits: Hit[] = [];
  for (let m = cursor.next(); !m.done && hits.length < most; m = cursor.next()) {
    const { from, to } = m.value;
    const line = view.state.doc.lineAt(from);
    hits.push({
      from,
      to,
      line: line.number,
      before: line.text.slice(Math.max(0, from - line.from - 28), from - line.from),
      match: view.state.sliceDoc(from, Math.min(to, line.to)),
      after: line.text.slice(Math.min(to, line.to) - line.from, Math.min(to, line.to) - line.from + 50),
    });
  }
  return hits;
}

export interface FindOptions {
  text: string;
  replace: string;
  caseSensitive: boolean;
  wholeWord: boolean;
  regexp: boolean;
}

const escapeRegex = (s: string) => s.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");

/// The query. Plain text ignoring case goes through a pattern of its own:
/// the editor folds case as English does, so "ı" wouldn't find "I".
export function queryOf(o: FindOptions): SearchQuery {
  if (o.regexp || o.caseSensitive) {
    return new SearchQuery({
      search: o.text,
      replace: o.replace,
      caseSensitive: o.caseSensitive,
      wholeWord: o.wholeWord,
      regexp: o.regexp,
      literal: !o.regexp,
    });
  }
  return new SearchQuery({
    search: escapeRegex(o.text).replace(/[iıIİ]/g, "[iıIİ]"),
    replace: o.replace.replace(/\$/g, "$$$$"),
    wholeWord: o.wholeWord,
    regexp: true,
  });
}

/// How many matches there are, and which the selection is on (from 1).
export function countOf(view: EditorView, q: SearchQuery, most = 9999): { all: number; at: number } {
  if (!q.valid) return { all: 0, at: 0 };
  const sel = view.state.selection.main;
  const cursor = q.getCursor(view.state) as Iterator<{ from: number; to: number }>;
  let all = 0;
  let at = 0;
  for (let m = cursor.next(); !m.done && all < most; m = cursor.next()) {
    all++;
    if (m.value.from === sel.from && m.value.to === sel.to) at = all;
  }
  return { all, at };
}

export function findPanel(view: EditorView): Panel {
  // The panel itself stays empty: the bar sits in the corner of the writing
  // area, not on the page, and doesn't grow with the page's zoom.
  const dom = document.createElement("div");
  dom.className = "find-host";
  let bar: ReturnType<typeof mount> | null = null;
  return {
    dom,
    top: true,
    mount() {
      bar = mount(FindBar, { target: view.dom.closest(".left") ?? dom, props: { view } });
    },
    destroy() {
      if (bar) void unmount(bar);
    },
  };
}
