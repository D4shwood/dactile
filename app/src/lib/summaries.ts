// Scene summaries (Alt+E), faint under their headings: never in the PDF,
// kept in the play's .dactile.toml (app/src-tauri/src/summary.rs).

import { StateEffect, StateField } from "@codemirror/state";
import { Decoration, EditorView, WidgetType, type DecorationSet } from "@codemirror/view";
import type { Summary } from "./api";

class Line extends WidgetType {
  constructor(readonly text: string) {
    super();
  }
  eq(o: Line) {
    return o.text === this.text;
  }
  toDOM() {
    const d = document.createElement("div");
    d.className = "cm-summary";
    d.textContent = this.text;
    return d;
  }
}

export const setSummaries = StateEffect.define<Summary[]>();

export const summaries = StateField.define<DecorationSet>({
  create: () => Decoration.none,
  update(deco, tr) {
    for (const e of tr.effects)
      if (e.is(setSummaries)) {
        const len = tr.state.doc.length;
        return Decoration.set(
          e.value
            .filter((s) => s.end <= len)
            .map((s) => Decoration.widget({ widget: new Line(s.text), block: true, side: 1 }).range(s.end)),
          true,
        );
      }
    return tr.docChanged ? deco.map(tr.changes) : deco;
  },
  provide: (f) => EditorView.decorations.from(f),
});
