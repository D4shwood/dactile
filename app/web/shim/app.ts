// `@tauri-apps/api/app` for the browser: the server's version.

export const getVersion = (): Promise<string> =>
  fetch("/api/version")
    .then((r) => r.json())
    .then((v: { version: string } | string) => (typeof v === "string" ? v : v.version));
