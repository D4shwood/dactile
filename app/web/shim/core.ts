// `@tauri-apps/api/core` for the browser: the window's commands go to
// dactile-server (server/) over HTTP. The interface in src/ is the desktop's
// own; what a desktop does with its own dialogs and programs becomes
// uploads, downloads and the browser's own tabs here.

import init, { call as callHere, names, set_lang } from "../wasm/dactile_wasm.js";
import { siteCommands, SignedOut } from "../../src/lib/site/commands";

const IMPORT = ".docx,.odt,.pdf,.md,.txt,.tex";

// The commands that only read the play's text run here, in WebAssembly
// (web-wasm/): the writer waits for no network as they type. Until they have
// loaded, and if they ever break, the server runs them as it runs the rest.
let here: Set<string> | null = null;
void init()
  .then(() => (here = new Set(JSON.parse(names()))))
  .catch((e) => console.warn("the browser's own commands didn't load", e));

/// After these the interface's language may be another: the browser's
/// commands are told before the answer goes on.
const SPEAKS = new Set(["home", "open", "set_setting", "create", "example"]);

async function follow() {
  if (!here) return;
  const lang = await fetch("/api/lang")
    .then((r) => (r.ok ? r.json() : null))
    .catch(() => null);
  if (lang) set_lang(lang);
}

/// A text-only command run here; `undefined` when it has to go to the server.
function local<T>(name: string, args: unknown): T | undefined {
  if (!here?.has(name)) return undefined;
  try {
    return JSON.parse(callHere(name, JSON.stringify(args ?? {}))) as T;
  } catch (e) {
    // The command's own error, as the server would give it.
    if (typeof e === "string") throw e;
    // Anything else is ours: the server takes over from here on.
    console.warn(`${name} failed in the browser`, e);
    here = null;
    return undefined;
  }
}

async function answer<T>(r: Response): Promise<T> {
  if (r.status === 401) {
    window.dispatchEvent(new Event("dactile-signed-out"));
    throw new SignedOut();
  }
  const body = await r.json().catch(() => r.statusText);
  // A command's error is its message, as Tauri gives it.
  if (!r.ok) throw typeof body === "string" ? body : JSON.stringify(body);
  return body as T;
}

/// What a browser lets a request carry and still finish it after its page
/// has gone (64 KiB for all such requests together), with room to spare.
const OUTLIVES = 48 * 1024;

export function post<T>(url: string, body: unknown, outlive = false): Promise<T> {
  const json = JSON.stringify(body ?? {});
  return fetch(url, {
    method: "POST",
    headers: { "content-type": "application/json", "x-dactile": "1" },
    body: json,
    // Three bytes a character at the most.
    keepalive: outlive && json.length * 3 < OUTLIVES,
  }).then(answer<T>);
}

/// A file the person picks, or null if they close the chooser.
function choose(accept: string): Promise<File | null> {
  return new Promise((done) => {
    const input = document.createElement("input");
    input.type = "file";
    input.accept = accept;
    input.addEventListener("change", () => done(input.files?.[0] ?? null));
    input.addEventListener("cancel", () => done(null));
    input.click();
  });
}

async function upload(kind: "play" | "import", accept: string): Promise<string | null> {
  const f = await choose(accept);
  if (!f) return null;
  return fetch(`/api/upload/${kind}`, {
    method: "POST",
    headers: { "x-dactile": "1", "x-name": encodeURIComponent(f.name) },
    body: f,
  }).then(answer<string>);
}

export const fileUrl = (path: string, download = false) =>
  `/api/file?path=${encodeURIComponent(path)}${download ? "&download=true" : ""}`;

function save(path: string) {
  const a = document.createElement("a");
  a.href = fileUrl(path, true);
  a.download = path.split("/").pop() ?? "";
  document.body.append(a);
  a.click();
  a.remove();
}

/// A file in a tab of its own; saved instead if the browser won't open one.
/// A folder can't be shown, and says so.
async function launch(path: string) {
  const r = await fetch(fileUrl(path), { method: "HEAD" });
  if (r.status === 400) throw await fetch(fileUrl(path)).then((r) => r.json());
  if (!window.open(fileUrl(path), "_blank", "noopener")) save(path);
}

/// The PDF printed from the browser's own viewer, in a frame of the page.
function print(path: string) {
  const frame = document.createElement("iframe");
  frame.style.cssText = "position:fixed;width:0;height:0;border:0";
  frame.src = fileUrl(path);
  frame.onload = () => {
    try {
      frame.contentWindow?.print();
    } catch {
      window.open(fileUrl(path), "_blank", "noopener");
    }
    setTimeout(() => frame.remove(), 60_000);
  };
  document.body.append(frame);
}

type Args = Record<string, any>;

/// The site's commands, sent from this page with its cookie.
const site = siteCommands((method, path, body, outlive) =>
  method === "GET" ? fetch(path).then(answer) : post(path, body, outlive),
);

export async function invoke<T>(cmd: string, args: Args = {}): Promise<T> {
  const done = (v?: unknown) => v as T;
  switch (cmd) {
    case "call":
      // The plays' folder is the server's: the settings form still sends it.
      if (args.name === "set_setting" && args.args?.key === "plays_dir") return done();
      {
        const answer = local<T>(args.name, args.args);
        if (answer !== undefined) return answer;
      }
      return site.call<T>(args.name, args.args).then(async (v) => {
        if (SPEAKS.has(args.name)) await follow();
        return v;
      });
    case "startup":
      return done(null);
    case "pick":
      return done(await upload("play", ".tex"));
    case "pick_import":
      return done(await upload("import", IMPORT));
    case "pick_text": {
      const f = await choose(".md,.txt");
      return done(f ? await f.text() : null);
    }
    case "pdf":
      return done(await fetch(fileUrl(args.path)).then((r) => (r.ok ? r.arrayBuffer() : answer(r))));
    case "launch":
      return done(await launch(args.path));
    case "reveal":
      return done(save(args.path));
    case "pdf_print":
      return done(print(args.path));
    case "full_screen":
      if (args.on && !document.fullscreenElement) await document.documentElement.requestFullscreen();
      if (!args.on && document.fullscreenElement) await document.exitFullscreen();
      return done();
    case "open_url":
      if (/^https?:\/\//.test(args.url)) window.open(args.url, "_blank", "noopener,noreferrer");
      return done();
    case "log":
      return post<T>("/api/log", args).catch(() => done());
    case "account_out":
      await post("/api/hesap/cik", {});
      location.href = "/";
      return done();
    case "sign_out":
      await post("/api/logout", {});
      location.reload();
      return done();
    default: {
      const answer = site.route<T>(cmd, args);
      if (answer) return answer;
      throw `${cmd}: not in the browser`;
    }
  }
}
