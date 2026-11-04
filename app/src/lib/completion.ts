// The menu a backslash opens, as in the terminal app: the play's characters
// (this scene's first), the class's commands and its other macros, ranked
// in Rust (core's `complete`) for what has been typed after it. A `#` at a
// line's start offers the headings; after `\xdo ` come the usual verbs.
// Ctrl+K opens it at the cursor.

import {
  autocompletion,
  closeCompletion,
  completionStatus,
  startCompletion,
  type Completion,
  type CompletionContext,
  type CompletionResult,
  type CompletionSection,
} from "@codemirror/autocomplete";
import { EditorView } from "@codemirror/view";
import type { Extension } from "@codemirror/state";
import { api, type Item } from "./api";

export interface CompleteHooks {
  tr: () => boolean;
  /// A suggestion that is a form: a new scene, act or character, at `at`.
  form: (kind: "scene" | "act" | "character", at: number) => void;
}

const sections = (tr: boolean): Record<Item["group"], CompletionSection> => ({
  scene: { name: tr ? "Bu sahnede" : "In this scene", rank: 0 },
  character: { name: tr ? "Karakterler" : "Characters", rank: 1 },
  command: { name: tr ? "Komutlar" : "Commands", rank: 2 },
  macro: { name: tr ? "Makrolar" : "Macros", rank: 3 },
});

function source(hooks: CompleteHooks) {
  return async (ctx: CompletionContext): Promise<CompletionResult | null> => {
    const line = ctx.state.doc.lineAt(ctx.pos);
    const before = line.text.slice(0, ctx.pos - line.from);
    const text = ctx.state.doc.toString();

    // `\xdo ` and the start of a verb.
    const verb = /\\[a-zA-Z]*do\{?\s+(\p{L}*)$/u.exec(before);
    if (verb) {
      const verbs = await api.verbs(text);
      if (ctx.aborted) return null;
      return {
        from: ctx.pos - verb[1].length,
        options: verbs.map((v) => ({ label: v, type: "text" })),
        validFor: /^\p{L}*$/u,
      };
    }

    const hash = /^\s*#(\p{L}*)$/u.exec(before);
    const back = /\\([\p{L}\d*]*)$/u.exec(before);
    const m = back ?? hash;
    if (!m) return null;
    const from = ctx.pos - m[1].length - 1;
    const items = await api.complete(text, ctx.pos, m[1], !back);
    if (ctx.aborted) return null;
    const sec = sections(hooks.tr());
    const options: Completion[] = items.map((it, i) => ({
      label: (back ? "\\" : "#") + it.key,
      detail: it.label,
      section: back ? sec[it.group] : undefined,
      // Rust ranked them: kept in its order.
      boost: 99 - Math.min(i, 198),
      type: it.action === "insert" ? (it.group === "macro" ? "function" : "variable") : "keyword",
      apply: (view, _c, a, b) => apply(view, it, a, b, hooks),
    }));
    return { from, to: ctx.pos, options, filter: false };
  };
}

function apply(view: EditorView, it: Item, from: number, to: number, hooks: CompleteHooks) {
  if (it.action !== "insert") {
    // The typed `\query` goes; the form writes the real thing.
    view.dispatch({ changes: { from, to, insert: "" }, selection: { anchor: from }, userEvent: "input.complete" });
    hooks.form(it.action, from);
    return;
  }
  view.dispatch({
    changes: { from, to, insert: it.text },
    selection: { anchor: from + it.cursor },
    userEvent: "input.complete",
  });
  // A direction's verb comes next.
  if (/do \s*$/.test(it.text)) requestAnimationFrame(() => startCompletion(view));
}

/// Where Ctrl+K put a backslash: taken back if the menu closes with
/// nothing chosen and nothing typed.
let opened: number | null = null;

/// Ctrl+K: a backslash at the cursor, as if typed, and the menu.
export function openMenu(view: EditorView, type: (view: EditorView) => void) {
  const at = view.state.selection.main.head;
  type(view);
  startCompletion(view);
  opened = at;
}

export function completion(hooks: CompleteHooks): Extension {
  return [
    autocompletion({
      override: [source(hooks)],
      activateOnTyping: true,
      icons: false,
      maxRenderedOptions: 60,
      closeOnBlur: true,
      tooltipClass: () => "dactile-menu",
    }),
    EditorView.updateListener.of((u) => {
      if (opened === null) return;
      if (completionStatus(u.state) !== null) return;
      const at = opened;
      opened = null;
      if (u.state.sliceDoc(at, at + 1) === "\\" && u.state.selection.main.head === at + 1)
        queueMicrotask(() => u.view.dispatch({ changes: { from: at, to: at + 1 }, userEvent: "delete" }));
    }),
  ];
}

export { closeCompletion };
