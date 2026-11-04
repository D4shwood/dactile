// F4 and F8, as in the terminal app. Scene focus: the other scenes fold to
// one line each (a click opens one), and the cursor's scene stays. Dim:
// all but the block the cursor is in goes faint.

import { StateEffect, StateField, type EditorState, type Range } from "@codemirror/state";
import { Decoration, EditorView, WidgetType, type DecorationSet } from "@codemirror/view";
import type { Card } from "./api";
import { blockAt, currentView } from "./editor";

interface Focus {
  on: boolean;
  dim: boolean;
  tr: boolean;
  cards: Card[];
  /// The scene folds, kept to rebuild only when the cursor changes scene.
  scene: number;
  deco: DecorationSet;
  here: DecorationSet;
}

export const setFocus = StateEffect.define<boolean>();
export const setDim = StateEffect.define<boolean>();
/// The board's cards, for where the scenes are.
export const setCards = StateEffect.define<{ cards: Card[]; tr: boolean }>();

class Folded extends WidgetType {
  constructor(
    readonly text: string,
    readonly at: number,
  ) {
    super();
  }
  eq(o: Folded) {
    return o.text === this.text && o.at === this.at;
  }
  toDOM(view: EditorView) {
    const d = document.createElement("div");
    d.className = "cm-folded";
    d.textContent = this.text;
    d.onmousedown = (e) => {
      e.preventDefault();
      view.dispatch({ selection: { anchor: this.at }, scrollIntoView: true });
      view.focus();
    };
    return d;
  }
  ignoreEvent() {
    return true;
  }
}

/// Each scene's range, from its heading to its last line.
function scenes(state: EditorState, cards: Card[]) {
  const len = state.doc.length;
  return cards
    .filter((c) => !c.act && c.end <= len)
    .map((c) => ({ from: c.at, to: c.end, card: c }));
}

function fold(state: EditorState, f: Focus): { deco: DecorationSet; scene: number } {
  if (!f.on) return { deco: Decoration.none, scene: -1 };
  const list = scenes(state, f.cards);
  const head = state.selection.main.head;
  const scene = list.findIndex((s) => head >= s.from && head <= s.to);
  const out: Range<Decoration>[] = [];
  list.forEach((s, i) => {
    if (i === scene || s.from >= s.to) return;
    const from = state.doc.lineAt(s.from).from;
    const to = state.doc.lineAt(s.to).to;
    const text = `▸ ${s.card.label}${s.card.title ? " — " + s.card.title : ""} · ${s.card.lines} ${f.tr ? "replik" : "lines"}`;
    out.push(Decoration.replace({ widget: new Folded(text, s.from), block: true }).range(from, to));
  });
  return { deco: Decoration.set(out, true), scene };
}

const hereLine = Decoration.line({ class: "cm-here" });

function here(state: EditorState, f: Focus): DecorationSet {
  if (!f.dim) return Decoration.none;
  const head = state.selection.main.head;
  const b = blockAt(currentView(state), head);
  const from = state.doc.lineAt(b ? Math.min(b.from, state.doc.length) : head).number;
  const to = state.doc.lineAt(b ? Math.min(b.to, state.doc.length) : head).number;
  const out: Range<Decoration>[] = [];
  for (let n = from; n <= to; n++) out.push(hereLine.range(state.doc.line(n).from));
  return Decoration.set(out);
}

export const focus = StateField.define<Focus>({
  create: () => ({ on: false, dim: false, tr: true, cards: [], scene: -1, deco: Decoration.none, here: Decoration.none }),
  update(f, tr) {
    let next = f;
    let refold = false;
    for (const e of tr.effects) {
      if (e.is(setFocus)) (next = { ...next, on: e.value }), (refold = true);
      if (e.is(setCards)) (next = { ...next, ...e.value }), (refold = true);
      if (e.is(setDim)) next = { ...next, dim: e.value };
    }
    if (!refold && tr.docChanged) next = { ...next, deco: next.deco.map(tr.changes) };
    if (next.on && !refold && tr.selection) {
      // The cursor went to another scene: it opens, this one folds.
      const s = fold(tr.state, next);
      if (s.scene !== next.scene) refold = true;
    }
    if (refold) next = { ...next, ...fold(tr.state, next) };
    if (next.dim || f.dim) next = { ...next, here: here(tr.state, next) };
    return next;
  },
  provide: (f) => [
    EditorView.decorations.from(f, (v) => v.deco),
    EditorView.decorations.from(f, (v) => v.here),
    EditorView.atomicRanges.of((v) => v.state.field(f).deco),
    EditorView.editorAttributes.from(f, (v) => ({ class: v.dim ? "cm-dimmed" : "" })),
  ],
});
