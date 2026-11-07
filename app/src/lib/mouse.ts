// Where a click lands, asked of the browser. CodeMirror finds it by
// scanning the line's pieces itself, and on lines that start with a hidden
// macro or a chip ((Selin, Ozan …) directions, \middir) WebKitGTK's
// rectangles lead it to the line's end. The browser's own caret lookup is
// right there, so clicks and drags use it, and CodeMirror's when it has
// nothing (a double click's word, the margins).

import { EditorSelection } from "@codemirror/state";
import { EditorView } from "@codemirror/view";

export function caretAt(view: EditorView, x: number, y: number): number | null {
  const r = document.caretRangeFromPoint?.(x, y);
  if (r && view.contentDOM.contains(r.startContainer)) {
    try {
      return view.posAtDOM(r.startContainer, r.startOffset);
    } catch {
      // Not in the document's text: below.
    }
  }
  return view.posAtCoords({ x, y }, false);
}

export const mouse = EditorView.mouseSelectionStyle.of((view, start) => {
  // Double and triple clicks (a word, a line) stay CodeMirror's.
  if (start.button !== 0 || start.detail > 1 || start.altKey || start.metaKey || start.ctrlKey) return null;
  let anchor = caretAt(view, start.clientX, start.clientY);
  if (anchor === null) return null;
  // Shift-click: from where the selection started.
  const from = start.shiftKey ? view.state.selection.main.anchor : null;
  return {
    get(cur) {
      // A click stays where it landed. The press can close a source block
      // above it, which moves the text, and the release would then find
      // another letter under the same spot.
      const still = Math.hypot(cur.clientX - start.clientX, cur.clientY - start.clientY) < 4;
      const head = still ? anchor! : (caretAt(view, cur.clientX, cur.clientY) ?? anchor!);
      return EditorSelection.single(from ?? anchor!, head);
    },
    update(u) {
      if (u.docChanged) anchor = u.changes.mapPos(anchor!);
      return false;
    },
  };
});
