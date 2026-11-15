// `@tauri-apps/api/window` for the browser. A tab closes without asking
// anyone: the writer saves when its tab is hidden instead (Writer.svelte),
// and a save small enough is sent to outlive the page (core.ts).

export type UnlistenFn = () => void;

export function getCurrentWindow() {
  return {
    onCloseRequested: async (_handler: (e: { preventDefault(): void }) => unknown): Promise<UnlistenFn> => () => {},
  };
}
