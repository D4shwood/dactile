// Spelling as you write: the words of the blocks in sight are sent to Rust
// (app/src-tauri/src/spell.rs) a moment after the last change, and the
// misspelt ones come back to be underlined. A right click on one offers
// the dictionary's suggestions and keeping it as the play's own word.

import { StateEffect, StateField, type EditorState, type Extension } from "@codemirror/state";
import { Decoration, EditorView, ViewPlugin, type DecorationSet, type ViewUpdate } from "@codemirror/view";
import { api } from "./api";

const setWords = StateEffect.define<[number, number][]>();

const misspelt = StateField.define<DecorationSet>({
  create: () => Decoration.none,
  update(deco, tr) {
    for (const e of tr.effects)
      if (e.is(setWords)) {
        const len = tr.state.doc.length;
        return Decoration.set(
          e.value.filter(([a, b]) => a < b && b <= len).map(([a, b]) => mark.range(a, b)),
          true,
        );
      }
    return tr.docChanged ? deco.map(tr.changes) : deco;
  },
  provide: (f) => EditorView.decorations.from(f),
});

const mark = Decoration.mark({ class: "cm-misspelt" });

export interface SpellHooks {
  tr: () => boolean;
  /// Why there is no checking (no dictionary), once.
  missing: (why: string) => void;
  on: () => boolean;
}

/// Checks what is in sight, a little after it changed or scrolled.
function checker(hooks: SpellHooks) {
  return ViewPlugin.fromClass(
    class {
      timer = 0;
      told = false;
      asked = 0;
      constructor(readonly view: EditorView) {
        this.soon(50);
      }
      update(u: ViewUpdate) {
        if (u.docChanged || u.viewportChanged) this.soon(u.docChanged ? 350 : 150);
      }
      soon(ms: number) {
        clearTimeout(this.timer);
        this.timer = window.setTimeout(() => this.check(), ms);
      }
      async check() {
        const view = this.view;
        if (!hooks.on()) {
          view.dispatch({ effects: setWords.of([]) });
          return;
        }
        const { from, to } = view.viewport;
        const doc = view.state.doc;
        const n = ++this.asked;
        const r = await api.misspelt(doc.toString(), from, to);
        // Only the last question's answer, and only for the text it was about.
        if (n !== this.asked || view.state.doc !== doc) return;
        if (r.kind === "loading") return this.soon(500);
        if (r.kind === "missing") {
          if (!this.told) hooks.missing(r.why);
          this.told = true;
          return;
        }
        view.dispatch({ effects: setWords.of(r.ranges) });
      }
      destroy() {
        clearTimeout(this.timer);
      }
    },
  );
}

/// The misspelt word at `pos`, if there is one.
export function wordAt(state: EditorState, pos: number): { from: number; to: number } | null {
  let found: { from: number; to: number } | null = null;
  state.field(misspelt).between(pos, pos, (from, to) => {
    found = { from, to };
    return false;
  });
  return found;
}

/// The next misspelt word after `pos`, going round to the top.
export function nextWord(state: EditorState, pos: number): { from: number; to: number } | null {
  let first: { from: number; to: number } | null = null;
  let next: { from: number; to: number } | null = null;
  state.field(misspelt).between(0, state.doc.length, (from, to) => {
    if (!first) first = { from, to };
    if (from > pos && !next) next = { from, to };
  });
  return next ?? first;
}

// ------------------------------------------------------------------ menu

let menu: HTMLElement | null = null;
/// When it opened: the scrolling that brought the word into sight doesn't
/// close it.
let menuBorn = 0;

function closeMenu() {
  menu?.remove();
  menu = null;
}

/// Suggestions for the misspelt word at `at`, by the mouse or the cursor.
export async function showMenu(
  view: EditorView,
  w: { from: number; to: number },
  x: number,
  y: number,
  tr: boolean,
  focus = false,
) {
  closeMenu();
  const word = view.state.sliceDoc(w.from, w.to);
  const box = document.createElement("div");
  box.className = "spell-menu";
  box.style.left = `${x}px`;
  box.style.top = `${y}px`;
  menu = box;
  menuBorn = performance.now();
  document.body.append(box);
  // From the keyboard: the arrows go through it, Escape leaves it.
  box.onkeydown = (e) => {
    const all = [...box.querySelectorAll("button")];
    const i = all.indexOf(document.activeElement as HTMLButtonElement);
    if (e.key === "Escape") (closeMenu(), view.focus());
    else if (e.key === "ArrowDown") all[(i + 1) % all.length]?.focus();
    else if (e.key === "ArrowUp") all[(i - 1 + all.length) % all.length]?.focus();
    else return;
    e.preventDefault();
    e.stopPropagation();
  };
  const item = (label: string, cls: string, act: () => void) => {
    const b = document.createElement("button");
    b.textContent = label;
    b.className = cls;
    b.onmousedown = (e) => e.preventDefault();
    b.onclick = () => {
      closeMenu();
      act();
      view.focus();
    };
    box.append(b);
    return b;
  };
  const wait = document.createElement("div");
  wait.className = "spell-quiet";
  wait.textContent = "…";
  box.append(wait);
  const sugg = await api.suggest(word);
  if (menu !== box) return;
  wait.remove();
  if (!sugg.length) {
    const none = document.createElement("div");
    none.className = "spell-quiet";
    none.textContent = tr ? "öneri yok" : "no suggestions";
    box.append(none);
  }
  for (const s of sugg)
    item(s, "spell-sugg", () => {
      // Only if the word is still there as it was.
      if (view.state.sliceDoc(w.from, w.to) !== word) return;
      view.dispatch({
        changes: { from: w.from, to: w.to, insert: s },
        selection: { anchor: w.from + s.length },
        userEvent: "input.spell",
      });
    });
  box.append(document.createElement("hr"));
  item(tr ? `+ «${word}» oyunun sözlüğüne` : `+ «${word}» to the play's words`, "spell-learn", async () => {
    await api.learn(word);
    // Checked again with the word known.
    recheck(view);
  });
  if (focus) box.querySelector("button")?.focus();
  // Kept inside the window.
  const r = box.getBoundingClientRect();
  if (r.bottom > window.innerHeight) box.style.top = `${Math.max(4, y - r.height)}px`;
  if (r.right > window.innerWidth) box.style.left = `${Math.max(4, window.innerWidth - r.width - 4)}px`;
}

/// Asks for a check now, after the play's words changed.
export function recheck(view: EditorView) {
  view.plugin(plugin!)?.soon(0);
}

let plugin: ReturnType<typeof checker> | null = null;

export function spelling(hooks: SpellHooks): Extension {
  plugin = checker(hooks);
  return [
    misspelt,
    plugin,
    EditorView.domEventHandlers({
      contextmenu(e, view) {
        const pos = view.posAtCoords({ x: e.clientX, y: e.clientY });
        const w = pos === null ? null : wordAt(view.state, pos);
        if (!w) return false;
        e.preventDefault();
        void showMenu(view, w, e.clientX, e.clientY, hooks.tr());
        return true;
      },
      mousedown() {
        closeMenu();
        return false;
      },
      keydown(e) {
        if (menu && e.key === "Escape") {
          closeMenu();
          return true;
        }
        return false;
      },
      scroll() {
        if (performance.now() - menuBorn > 300) closeMenu();
        return false;
      },
    }),
  ];
}

export { closeMenu };
