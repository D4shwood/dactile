// `@tauri-apps/api/event` for the browser. The one event the interface
// listens to is the OCR's progress; here it is asked for while it runs.

export type UnlistenFn = () => void;

export async function listen<T>(event: string, handler: (e: { payload: T }) => void): Promise<UnlistenFn> {
  if (event !== "ocr") return () => {};
  let last = "";
  const timer = setInterval(async () => {
    const r = await fetch("/api/progress").catch(() => null);
    if (!r?.ok) return;
    const now = await r.json();
    if (JSON.stringify(now) !== last && now[1] > 0) {
      last = JSON.stringify(now);
      handler({ payload: now as T });
    }
  }, 700);
  return () => clearInterval(timer);
}
