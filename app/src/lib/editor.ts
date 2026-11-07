// The page: CodeMirror over the play's own text. The Rust side says how each
// block reads (view.rs); here that becomes decorations. A chip covers a
// macro (`\se` shows as SELİN) and the cursor steps over it as one piece.
// A block opens as its source when its chip is clicked, when backspace
// reaches a chip, or when a backslash is typed; it closes again once the
// cursor leaves it. Typed characters are escaped as the terminal app does
// (core/src/buffer.rs), so a % prints as a %.

import { EditorState, StateEffect, StateField, type Extension, type Range } from "@codemirror/state";
import {
  Decoration,
  type DecorationSet,
  EditorView,
  WidgetType,
  drawSelection,
  keymap,
} from "@codemirror/view";
import { defaultKeymap, history, historyKeymap, indentWithTab } from "@codemirror/commands";
import { highlightSelectionMatches, search, searchKeymap } from "@codemirror/search";
import type { Block, Cover, View } from "./api";

// ------------------------------------------------------------------ state

/// A new view from the Rust side, for the document as it is now.
export const setView = StateEffect.define<View>();
/// Every block shown as its source.
export const setSource = StateEffect.define<boolean>();
/// The block at a position, or a range (the cover), opened as its source.
const openRaw = StateEffect.define<number | { from: number; to: number }>();

interface Page {
  view: View | null;
  /// The view is for an older text: positions are mapped until a new one comes.
  stale: boolean;
  source: boolean;
  /// The block open as its source: its range, mapped through edits.
  raw: { from: number; to: number } | null;
}

const page = StateField.define<Page>({
  create: () => ({ view: null, stale: false, source: false, raw: null }),
  update(p, tr) {
    let next = p;
    if (tr.docChanged) {
      next = {
        ...next,
        stale: true,
        raw: next.raw && {
          from: tr.changes.mapPos(next.raw.from, -1),
          to: tr.changes.mapPos(next.raw.to, 1),
        },
      };
    }
    for (const e of tr.effects) {
      if (e.is(setView)) next = { ...next, view: e.value, stale: false };
      if (e.is(setSource)) next = { ...next, source: e.value };
      if (e.is(openRaw)) {
        const v = e.value;
        const b = typeof v === "number" ? blockAt(next.view, v) : v;
        next = { ...next, raw: b ? { from: b.from, to: b.to } : null };
      }
    }
    // The cursor left the raw block: it reads as the page again.
    const head = tr.state.selection.main.head;
    if (next.raw && !next.stale && (head < next.raw.from || head > next.raw.to)) {
      next = { ...next, raw: null };
    }
    return next;
  },
});

export function blockAt(v: View | null, pos: number): Block | null {
  if (!v) return null;
  let lo = 0;
  let hi = v.blocks.length - 1;
  while (lo <= hi) {
    const mid = (lo + hi) >> 1;
    const b = v.blocks[mid];
    if (pos < b.from) hi = mid - 1;
    else if (pos > b.to) lo = mid + 1;
    else return b;
  }
  return null;
}

// ------------------------------------------------------------------ widgets

class Chip extends WidgetType {
  constructor(
    readonly label: string,
    readonly cls: string,
    readonly at: number,
  ) {
    super();
  }
  eq(o: Chip) {
    return o.label === this.label && o.cls === this.cls;
  }
  toDOM() {
    const s = document.createElement("span");
    s.className = "cm-chip " + this.cls.split(" ").map((c) => "cm-c-" + c).join(" ");
    s.textContent = this.label;
    s.dataset.at = String(this.at);
    return s;
  }
  ignoreEvent() {
    return false;
  }
}

class CoverCard extends WidgetType {
  constructor(readonly c: Cover) {
    super();
  }
  eq(o: CoverCard) {
    return JSON.stringify(o.c) === JSON.stringify(this.c);
  }
  toDOM() {
    const d = document.createElement("div");
    d.className = "cm-cover";
    const t = document.createElement("div");
    t.className = "cm-cover-title";
    t.textContent = this.c.title || "—";
    const a = document.createElement("div");
    a.className = "cm-cover-author";
    a.textContent = this.c.author;
    const list = document.createElement("ul");
    list.className = "cm-cover-cast";
    for (const r of this.c.cast) {
      const li = document.createElement("li");
      if (r.kind === "person") {
        li.textContent = r.name;
      } else {
        // The members, a brace as tall as they are, and the group's name.
        li.className = "cm-cover-group";
        const members = document.createElement("div");
        members.className = "cm-cover-members";
        for (const m of r.members) {
          const n = document.createElement("div");
          n.textContent = m;
          members.append(n);
        }
        const brace = document.createElementNS("http://www.w3.org/2000/svg", "svg");
        brace.setAttribute("class", "cm-cover-brace");
        brace.setAttribute("viewBox", "0 0 10 100");
        brace.setAttribute("preserveAspectRatio", "none");
        const path = document.createElementNS("http://www.w3.org/2000/svg", "path");
        path.setAttribute("d", "M1 1 Q5 1 5 8 L5 42 Q5 50 9 50 Q5 50 5 58 L5 92 Q5 99 1 99");
        path.setAttribute("vector-effect", "non-scaling-stroke");
        brace.append(path);
        const name = document.createElement("div");
        name.className = "cm-cover-group-name";
        name.textContent = r.name;
        // The brace in a box of no height of its own: it takes the members'.
        const box = document.createElement("span");
        box.className = "cm-cover-brace-box";
        box.append(brace);
        li.append(members, box, name);
      }
      list.append(li);
    }
    const hint = document.createElement("div");
    hint.className = "cm-cover-hint";
    hint.textContent = "kapak ve kişiler · düzenlemek için tıkla";
    d.append(t, a, list, hint);
    d.dataset.at = "0";
    return d;
  }
  ignoreEvent() {
    return false;
  }
}

// ------------------------------------------------------------------ decorations

function decorate(state: EditorState): DecorationSet {
  const p = state.field(page);
  const v = p.view;
  if (!v || p.source) return Decoration.none;
  const doc = state.doc;
  const out: Range<Decoration>[] = [];
  const head = state.selection.main.head;
  const len = doc.length;
  const ok = (n: number) => n >= 0 && n <= len;

  if (v.cover && ok(v.cover.to)) {
    const open = p.raw !== null && p.raw.from === 0 && p.raw.to >= v.cover.to;
    if (!open && v.cover.to > 0) {
      // To the end of the `\begin{document}` line, so no empty line is left.
      const to = doc.lineAt(v.cover.to).to;
      out.push(Decoration.replace({ widget: new CoverCard(v.cover), block: true }).range(0, Math.min(to, len)));
    }
  }
  if (v.tail && ok(v.tail.end) && v.tail.start < v.tail.end) {
    const from = doc.lineAt(v.tail.start).from;
    if (head < from) out.push(Decoration.replace({ block: true }).range(Math.max(from - 1, 0), v.tail.end));
  }

  for (const b of v.blocks) {
    if (!ok(b.to)) break;
    const raw = p.raw !== null && b.from <= p.raw.to && b.to >= p.raw.from;
    const cls = "cm-b-" + b.cls + (raw ? " cm-raw" : "");
    for (let pos = b.from; pos <= b.to; ) {
      const line = doc.lineAt(pos);
      out.push(Decoration.line({ class: cls }).range(line.from));
      pos = line.to + 1;
    }
    if (raw) continue;
    for (const m of b.marks) {
      if (!ok(m.to)) continue;
      if (m.label === null) {
        out.push(Decoration.mark({ class: "cm-m-" + m.cls }).range(m.from, m.to));
      } else if (m.cls === "hide" && m.label === "") {
        if (m.from < m.to) out.push(Decoration.replace({}).range(m.from, m.to));
      } else if (m.from < m.to) {
        out.push(Decoration.replace({ widget: new Chip(m.label, m.cls, m.from) }).range(m.from, m.to));
      } else {
        out.push(Decoration.widget({ widget: new Chip(m.label, m.cls, m.from), side: 1 }).range(m.from));
      }
    }
  }
  return Decoration.set(out, true);
}

const decorations = StateField.define<DecorationSet>({
  create: (s) => decorate(s),
  update(deco, tr) {
    const before = tr.startState.field(page);
    const after = tr.state.field(page);
    const rebuilt =
      before.view !== after.view ||
      before.source !== after.source ||
      before.raw !== after.raw ||
      (tr.selection && !after.stale && coverOrTailMoved(tr.startState, tr.state));
    if (rebuilt && !after.stale) return decorate(tr.state);
    // Until the new view comes, the old marks move with the text.
    return tr.docChanged ? deco.map(tr.changes) : deco;
  },
  provide: (f) => [EditorView.decorations.from(f), EditorView.atomicRanges.of((v) => v.state.field(f))],
});

/// The tail hides only while the cursor is above it.
function coverOrTailMoved(a: EditorState, b: EditorState) {
  const v = b.field(page).view;
  if (!v?.tail) return false;
  const before = a.selection.main.head >= v.tail.start;
  const now = b.selection.main.head >= v.tail.start;
  return before !== now;
}

// ------------------------------------------------------------------ notes

/// The notes' places in the text, underlined; the picked one stronger.
export const setNotes = StateEffect.define<{ from: number; to: number; i: number; done: boolean }[]>();
export const pickNote = StateEffect.define<number | null>();

const noteMarks = StateField.define<{ deco: DecorationSet; picked: number | null; list: { from: number; to: number; i: number; done: boolean }[] }>({
  create: () => ({ deco: Decoration.none, picked: null, list: [] }),
  update(v, tr) {
    let list = v.list;
    let picked = v.picked;
    let changed = false;
    for (const e of tr.effects) {
      if (e.is(setNotes)) (list = e.value), (changed = true);
      if (e.is(pickNote)) (picked = e.value), (changed = true);
    }
    if (!changed) return tr.docChanged ? { ...v, deco: v.deco.map(tr.changes) } : v;
    const len = tr.state.doc.length;
    const deco = Decoration.set(
      list
        .filter((n) => n.from < n.to && n.to <= len)
        .map((n) =>
          Decoration.mark({
            class: "cm-note" + (n.done ? " cm-note-done" : "") + (n.i === picked ? " cm-note-picked" : ""),
            attributes: { "data-note": String(n.i) },
          }).range(n.from, n.to),
        ),
      true,
    );
    return { deco, picked, list };
  },
  provide: (f) => EditorView.decorations.from(f, (v) => v.deco),
});

// ------------------------------------------------------------------ typing

function rendered(state: EditorState, pos: number) {
  const p = state.field(page);
  if (p.source || !p.view) return null;
  const b = blockAt(p.view, pos);
  if (p.raw && b && b.from <= p.raw.to && b.to >= p.raw.from) return null;
  return b;
}

const ESCAPES: Record<string, string> = {
  "%": "\\%",
  "#": "\\#",
  $: "\\$",
  "&": "\\&",
  "{": "\\{",
  "}": "\\}",
  "^": "\\^{}",
  "~": "\\textasciitilde{}",
};

function escapeTyped(c: string, dialogue: boolean) {
  if (c === "_") return dialogue ? "_" : "\\_";
  return ESCAPES[c] ?? c;
}

function lineStart(state: EditorState, pos: number) {
  const line = state.doc.lineAt(pos);
  return state.doc.sliceString(line.from, pos).trim() === "";
}

/// A backslash typed at the cursor, as the keyboard would.
export function typeBackslash(view: EditorView) {
  const { from, to } = view.state.selection.main;
  view.dispatch({
    changes: { from, to, insert: "\\" },
    selection: { anchor: from + 1 },
    effects: openRaw.of(from),
    userEvent: "input.type",
  });
}

const typing = EditorView.inputHandler.of((view, from, to, text) => {
  const b = rendered(view.state, from);
  if (!b || text.length === 0) return false;
  // A backslash opens the block as source: a macro is being written.
  if (text === "\\") {
    view.dispatch({
      changes: { from, to, insert: "\\" },
      selection: { anchor: from + 1 },
      effects: openRaw.of(from),
      userEvent: "input.type",
    });
    return true;
  }
  // `#perde` and `#sahne` start a line: a # there stays a #.
  if (text === "#" && lineStart(view.state, from)) return false;
  const dialogue = b.cls === "line" || b.cls === "direction";
  const out = [...text].map((c) => escapeTyped(c, dialogue)).join("");
  if (out === text) return false;
  view.dispatch({
    changes: { from, to, insert: out },
    selection: { anchor: from + out.length },
    userEvent: "input.type",
  });
  return true;
});

function escapePasted(s: string, dialogue: boolean) {
  return [...s.replace(/\r\n/g, "\n")]
    .map((c) => (c === "\\" ? "\\textbackslash{}" : c === "\t" ? " " : escapeTyped(c, dialogue)))
    .join("");
}

const pasting = EditorView.domEventHandlers({
  paste(e, view) {
    const text = e.clipboardData?.getData("text/plain");
    const sel = view.state.selection.main;
    const b = rendered(view.state, sel.from);
    // Pasted TeX (it has macros in it) goes in as it is.
    if (!text || !b || /\\[a-zA-Z]/.test(text)) return false;
    const dialogue = b.cls === "line" || b.cls === "direction";
    const insert = escapePasted(text, dialogue);
    view.dispatch({
      changes: { from: sel.from, to: sel.to, insert },
      selection: { anchor: sel.from + insert.length },
      userEvent: "input.paste",
    });
    e.preventDefault();
    return true;
  },
  mousedown(e, view) {
    // A chip or the cover clicked: its block opens as source.
    const t = (e.target as HTMLElement).closest("[data-at]") as HTMLElement | null;
    if (!t) return false;
    const at = Number(t.dataset.at);
    const cover = view.state.field(page).view?.cover;
    if (cover && t.classList.contains("cm-cover")) {
      // The cover is not a block: it opens as a whole.
      view.dispatch({
        effects: openRaw.of({ from: 0, to: cover.to }),
        selection: { anchor: 0 },
      });
    } else {
      view.dispatch({ effects: openRaw.of(at), selection: { anchor: at } });
    }
    e.preventDefault();
    return true;
  },
});

/// Backspace or delete against a chip: the block opens as source instead.
function againstChip(view: EditorView, back: boolean) {
  const sel = view.state.selection.main;
  if (!sel.empty) return false;
  const b = rendered(view.state, sel.head);
  if (!b) return false;
  const pos = sel.head;
  const hit = b.marks.find((m) => m.label !== null && (back ? m.to === pos : m.from === pos) && m.from < m.to);
  if (!hit) return false;
  view.dispatch({ effects: openRaw.of(pos) });
  return true;
}

// ------------------------------------------------------------------ dialogue

/// Enter at the end of a line of dialogue: a new line for the one who
/// answers, the other speaker of the exchange.
function newLine(view: EditorView) {
  const sel = view.state.selection.main;
  const p = view.state.field(page);
  if (!sel.empty || p.stale) return false;
  const b = rendered(view.state, sel.head);
  if (!b || b.cls !== "line" || !b.alias || sel.head !== b.to) return false;
  const v = p.view!;
  // An empty line of dialogue: Enter ends the dialogue instead.
  const name = b.marks[0];
  if (view.state.doc.sliceString(name.to, b.to).trim() === "") {
    view.dispatch({ changes: { from: b.from, to: b.to, insert: "" }, userEvent: "delete" });
    return true;
  }
  const i = v.blocks.indexOf(b);
  let next: string | null = null;
  for (let k = i - 1; k >= 0 && !next; k--) {
    const o = v.blocks[k];
    if (o.cls === "scene" || o.cls === "act") break;
    if (o.alias && o.alias !== b.alias) next = o.alias;
  }
  next ??= v.cast.find((c) => c.alias !== b.alias)?.alias ?? b.alias;
  const insert = `\n\\${next} `;
  view.dispatch({
    changes: { from: sel.head, insert },
    selection: { anchor: sel.head + insert.length },
    userEvent: "input",
  });
  return true;
}

/// Tab right after the speaker: the next character speaks this line.
function nextSpeaker(view: EditorView) {
  const sel = view.state.selection.main;
  const p = view.state.field(page);
  if (!sel.empty || p.stale) return false;
  const b = rendered(view.state, sel.head);
  if (!b || b.cls !== "line" || !b.alias) return false;
  const name = b.marks[0];
  const rest = view.state.doc.sliceString(name.to, sel.head);
  if (rest.trim() !== "") return false;
  const cast = p.view!.cast;
  const i = cast.findIndex((c) => c.alias === b.alias);
  const next = cast[(i + 1) % cast.length];
  view.dispatch({
    changes: { from: name.from, to: name.to, insert: "\\" + next.alias },
    userEvent: "input",
  });
  return true;
}

// ------------------------------------------------------------------ setup

export interface Hooks {
  /// The text changed: a new view is wanted.
  changed: (state: EditorState) => void;
  save: () => void;
}

export function extensions(hooks: Hooks): Extension {
  return [
    page,
    decorations,
    noteMarks,
    // Our own caret: WebKit's blinks only if the desktop says so.
    drawSelection({ cursorBlinkRate: 1100 }),
    history(),
    search({ top: true }),
    highlightSelectionMatches(),
    typing,
    pasting,
    EditorView.lineWrapping,
    keymap.of([
      { key: "Mod-s", run: () => (hooks.save(), true), preventDefault: true },
      { key: "Enter", run: newLine },
      { key: "Tab", run: nextSpeaker },
      { key: "Backspace", run: (v) => againstChip(v, true) },
      { key: "Delete", run: (v) => againstChip(v, false) },
      // F3 is the typewriter here, not CodeMirror's find-next (which opens
      // the search panel when there is nothing to find).
      ...searchKeymap.filter((b) => b.key !== "F3"),
      ...historyKeymap,
      ...defaultKeymap,
      indentWithTab,
    ]),
    EditorView.updateListener.of((u) => {
      if (u.docChanged) hooks.changed(u.state);
    }),
  ];
}

export function isStale(state: EditorState) {
  return state.field(page).stale;
}

export function currentView(state: EditorState) {
  return state.field(page).view;
}

/// Inserts a snippet with `‸` for the cursor: on a line of its own after
/// the cursor's line (`block`), or right at the cursor. Never in the cover.
export function insertSnippet(view: EditorView, s: string, block: boolean) {
  const state = view.state;
  const pos = state.selection.main.head;
  let at = pos;
  let text = s;
  if (block) {
    const line = state.doc.lineAt(pos);
    if (line.text.trim() === "") {
      at = line.from;
    } else {
      at = line.to;
      text = "\n\n" + s;
      // A line right below gets a blank line between.
      const below = line.number < state.doc.lines ? state.doc.line(line.number + 1).text : "";
      if (below.trim() !== "") text += "\n";
    }
    const cover = state.field(page).view?.cover;
    if (cover && at < cover.to) {
      at = state.doc.lineAt(Math.min(cover.to, state.doc.length)).to;
      text = "\n\n" + s;
    }
  }
  const mark = text.indexOf("‸");
  const insert = text.replace("‸", "");
  view.dispatch({
    changes: { from: at, insert },
    selection: { anchor: at + (mark < 0 ? insert.length : mark) },
    effects: s.startsWith("\\") || s.startsWith("#") ? openRaw.of(at + 1) : [],
    userEvent: "input",
    scrollIntoView: true,
  });
  view.focus();
}

/// The cover opened as its source, the cursor at its start.
export function openCover(view: EditorView) {
  const cover = view.state.field(page).view?.cover;
  if (!cover) return;
  view.dispatch({ effects: openRaw.of({ from: 0, to: cover.to }), selection: { anchor: 0 }, scrollIntoView: true });
  view.focus();
}
