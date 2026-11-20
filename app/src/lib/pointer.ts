// What the pointer does across the app: one drag followed to its end, and a
// floating box kept on the screen.

/// Follows the drag that `e` starts on its element until the pointer is
/// let go or the drag is cancelled (a touch taken over by scrolling, the
/// window losing it): `done` runs either way, so nothing stays half-dragged.
export function drag(e: PointerEvent, move: (m: PointerEvent) => void, done: (m: PointerEvent) => void = () => {}) {
  const el = e.currentTarget as HTMLElement;
  // A pointer already gone cannot be captured: the drag still ends on its "up".
  try {
    el.setPointerCapture(e.pointerId);
  } catch {}
  const end = (m: PointerEvent) => {
    el.removeEventListener("pointermove", move);
    el.removeEventListener("pointerup", end);
    el.removeEventListener("pointercancel", end);
    done(m);
  };
  el.addEventListener("pointermove", move);
  el.addEventListener("pointerup", end);
  el.addEventListener("pointercancel", end);
}

/// A fixed box moved back inside the window, 8px from its edges, measured
/// rather than guessed: `use:onScreen` on a menu or popup.
export function keepOnScreen(box: HTMLElement) {
  const r = box.getBoundingClientRect();
  const left = Math.max(8, Math.min(r.left, innerWidth - r.width - 8));
  const top = r.bottom > innerHeight - 8 ? Math.max(8, r.top - r.height) : r.top;
  if (left !== r.left) box.style.left = `${left}px`;
  if (top !== r.top) box.style.top = `${top}px`;
}

export function onScreen(box: HTMLElement) {
  keepOnScreen(box);
}
