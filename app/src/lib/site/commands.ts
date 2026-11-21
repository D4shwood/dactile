// The window's commands as the site's gate takes them: the backend's own
// over `/api/call/<name>`, and the gate's (people, projects, the account)
// over their routes. Shared by the browser (app/web/shim/), which sends
// with `fetch`, and by a desktop linked to a site, which sends through its
// Rust side with its token. Neither the files a desktop opens nor a
// browser's tabs are here: each keeps those itself.

/// The site's API this interface speaks (server/src/lib.rs, `API`): a site
/// that says another one isn't opened from a desktop until it is updated.
export const API = 1;

/// One request to the site: the answer's body, or the command's error (a
/// string) thrown. A request the site no longer lets in throws `SignedOut`.
export type Send = (method: "GET" | "POST", path: string, body?: unknown, outlive?: boolean) => Promise<unknown>;

export class SignedOut extends Error {
  constructor() {
    super("signed out");
  }
}

type Args = Record<string, any>;

const id = (args: Args) => encodeURIComponent(args.id);

/// The gate's own routes, by command.
const ROUTES: Record<string, (a: Args) => ["GET" | "POST", string, unknown?]> = {
  import_read: (a) => ["POST", "/api/import_read", a],
  people: () => ["GET", "/api/people"],
  person_invite: (a) => ["POST", "/api/people/davet", { mail: a.mail, name: a.name }],
  person_uninvite: (a) => ["POST", "/api/people/davet/geri", { mail: a.mail }],
  person_reset: (a) => ["POST", `/api/people/${id(a)}/reset`, {}],
  person_remove: (a) => ["POST", `/api/people/${id(a)}/remove`, {}],
  projects: () => ["GET", "/api/projects"],
  project_people: () => ["GET", "/api/projects/people"],
  project_create: (a) => ["POST", "/api/projects", { name: a.name }],
  project_member: (a) => ["POST", `/api/projects/${id(a)}/member`, { who: a.who, role: a.role }],
  project_mode: (a) => ["POST", `/api/projects/${id(a)}/mode`, { mode: a.mode }],
  project_rename: (a) => ["POST", `/api/projects/${id(a)}/name`, { name: a.name }],
  project_leave: (a) => ["POST", `/api/projects/${id(a)}/leave`, {}],
  account: () => ["GET", "/api/hesap"],
  account_name: (a) => ["POST", "/api/hesap", { name: a.name }],
  account_password: (a) => ["POST", "/api/hesap/parola", { old: a.old, new: a.new }],
  account_mail: (a) => ["POST", "/api/hesap/eposta", { mail: a.mail }],
  devices: () => ["GET", "/api/cihazlar"],
  device_off: (a) => ["POST", `/api/cihazlar/${id(a)}/kaldir`, {}],
};

/// The site's commands over `send`.
export function siteCommands(send: Send) {
  // The play that is open, as the person's server knows it. Their server is
  // stopped when they have been away a while and started again when they
  // come back, with no play open: the next command that needs one opens it.
  let openPath: string | null = null;

  /// One of the backend's commands, run by the person's server.
  async function call<T>(name: string, args: unknown): Promise<T> {
    // A save is finished even if the window closes on it.
    const once = () => send("POST", `/api/call/${name}`, args, name === "save") as Promise<T>;
    let v: T;
    try {
      v = await once();
    } catch (e) {
      if (e !== "no play open" || !openPath || name === "open") throw e;
      await send("POST", "/api/call/open", { path: openPath });
      v = await once();
    }
    if (name === "open") openPath = (v as { path: string }).path;
    if (name === "close") openPath = null;
    return v;
  }

  /// A command of the gate's own; undefined if `cmd` isn't one.
  function route<T>(cmd: string, args: Args = {}): Promise<T> | undefined {
    if (cmd === "door") {
      return (send("GET", "/api/me") as Promise<{ name: string | null; role?: string }>).then(
        (me) => ({ name: me.name, owner: me.role === "owner" }) as T,
      );
    }
    const r = ROUTES[cmd];
    if (!r) return undefined;
    const [method, path, body] = r(args);
    return send(method, path, body) as Promise<T>;
  }

  return { call, route };
}
