// Questions and notices where the user is looking: at the pointer's last
// press, or beside what has the keyboard's focus; in the middle only when
// neither is known. `Ask.svelte` draws them; App mounts it once.
//
//   if (await ask({ title: "Delete it?", yes: "Delete", no: "Cancel", danger: true })) …
//   notify({ text: "In the trash.", action: { label: "Undo", run: restore } });

export type Where = { x: number; y: number } | Element | Event | null | undefined;

let next = 0;

export interface Question {
  title: string;
  detail?: string;
  yes: string;
  no: string;
  /// It can't be undone, or it takes something away: Cancel is the default.
  danger: boolean;
  at: { x: number; y: number } | null;
  answer: (yes: boolean) => void;
  id: number;
}

export interface Notice {
  text: string;
  warn: boolean;
  action?: { label: string; run: () => void };
  at: { x: number; y: number } | null;
  id: number;
}

// Kept as proxies: compared by id, never by identity.
export const shown = $state<{ question: Question | null; notice: Notice | null }>({ question: null, notice: null });

/// Where the pointer last pressed, and when: a question soon after is about it.
let pressed: { x: number; y: number; t: number } | null = null;
const FRESH = 4000;

if (typeof window !== "undefined") {
  window.addEventListener("pointerdown", (e) => (pressed = { x: e.clientX, y: e.clientY, t: Date.now() }), { capture: true });
}

/// A point for `where`: the event's or element's own, else the last press,
/// else what has focus; null for the middle.
function place(where: Where): { x: number; y: number } | null {
  if (where instanceof Event) {
    if ("clientX" in where && (where as MouseEvent).clientX) return { x: (where as MouseEvent).clientX, y: (where as MouseEvent).clientY };
    if (where.currentTarget instanceof Element) where = where.currentTarget;
    else where = null;
  }
  if (where instanceof Element) {
    const r = where.getBoundingClientRect();
    return { x: r.left, y: r.bottom + 4 };
  }
  if (where) return where;
  if (pressed && Date.now() - pressed.t < FRESH) return { x: pressed.x, y: pressed.y };
  const f = document.activeElement;
  if (f && f !== document.body) {
    const r = f.getBoundingClientRect();
    if (r.width || r.height) return { x: r.left, y: r.bottom + 4 };
  }
  return null;
}

export function ask(o: { title: string; detail?: string; yes: string; no: string; danger?: boolean; at?: Where }): Promise<boolean> {
  shown.question?.answer(false);
  const id = ++next;
  return new Promise((resolve) => {
    const q: Question = {
      id,
      title: o.title,
      detail: o.detail,
      yes: o.yes,
      no: o.no,
      danger: o.danger ?? false,
      at: place(o.at),
      answer: (yes) => {
        if (shown.question?.id === id) shown.question = null;
        resolve(yes);
      },
    };
    shown.question = q;
  });
}

export function notify(o: { text: string; warn?: boolean; action?: { label: string; run: () => void }; at?: Where; ms?: number }) {
  const n: Notice = { text: o.text, warn: o.warn ?? false, action: o.action, at: place(o.at), id: ++next };
  shown.notice = n;
  setTimeout(() => {
    if (shown.notice?.id === n.id) shown.notice = null;
  }, o.ms ?? (o.action ? 8000 : 4000));
}
