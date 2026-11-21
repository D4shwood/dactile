// A desktop linked to a site (src-tauri/src/site.rs): the site's commands
// sent through the Rust side with this computer's token, and the switch
// that sends the window's commands there while a site's play is open.
// Nothing here runs in the browser, where the page is the site's own.

import { invoke as tauri } from "@tauri-apps/api/core";
import { siteCommands, SignedOut } from "./commands";
import { setRoomAddress } from "../live";

/// The site this computer is linked to, as the window is told of it.
export interface SiteLink {
  url: string;
  id: string;
  name: string;
}

/// What a site says of itself.
export interface SiteDoor {
  version: string;
  api: number;
  stage: string;
}

/// The site no longer lets this computer in: the window says so.
export const SITE_OUT = "dactile-site-out";

export const site = siteCommands(async (method, path, body) => {
  try {
    return await tauri("site_request", { method, path, body: body ?? null });
  } catch (e) {
    if (e === "signed out") {
      window.dispatchEvent(new Event(SITE_OUT));
      throw new SignedOut();
    }
    throw e;
  }
});

export const siteLink = () => tauri<SiteLink | null>("site_status");
export const siteDoor = (url: string) => tauri<SiteDoor>("site_door", { url });
/// The code to show, and the site's page opened in the browser for it.
export const siteConnect = (url: string) => tauri<{ code: string; page: string }>("site_connect", { url });
/// Until someone says yes there; the link, kept.
export const siteWait = () => tauri<SiteLink>("site_wait");
export const siteForget = () => tauri<void>("site_forget");
/// The site's desktop, if newer than this one: its version. Null in a
/// build that can't update itself (not an AppImage).
export const siteUpdate = () => tauri<string | null>("site_update");
/// Puts the site's desktop in this one's place and opens it anew.
export const siteUpdateInstall = () => tauri<void>("site_update_install");

/// Whether the play open now is the site's.
let onSite = false;
/// The backend's commands that run here even then: they read only their
/// arguments. With them the settings, which are this computer's.
let here: Set<string> | null = null;
const STAYS = ["settings", "set_setting", "keys_view"];

export const isOnSite = () => onSite;

/// From here on the window's commands go to the site (`on`), or stay here.
export async function useSite(on: boolean) {
  if (on && !here) here = new Set([...(await tauri<string[]>("text_only")), ...STAYS]);
  if (on) setRoomAddress((path) => tauri<string>("site_room", { path }));
  onSite = on;
}

type Args = Record<string, any>;

/// A file of the site's, kept on this computer for one of its programs.
const fetched = (a: Args) => tauri<string>("site_fetch", { path: a.path });

/// The desktop's own commands, for a play of the site's.
const OWN: Record<string, (a: Args) => Promise<unknown>> = {
  pdf: (a) => tauri("site_file", { path: a.path }),
  launch: async (a) => tauri("launch", { path: await fetched(a) }),
  reveal: async (a) => tauri("reveal", { path: await fetched(a) }),
  pdf_print: async (a) => tauri("pdf_print", { ...a, path: await fetched(a) }),
  pick: () => tauri("site_upload", { kind: "play" }),
  pick_import: () => tauri("site_upload", { kind: "import" }),
};

/// Where `name` goes while a site's play is open: its answer, or undefined
/// to run it here as always.
export function onSiteInvoke<T>(name: string, args: Args, desktop: boolean): Promise<T> | undefined {
  if (!onSite) return undefined;
  if (OWN[name]) return OWN[name](args) as Promise<T>;
  if (desktop) return site.route<T>(name, args);
  if (here?.has(name)) return undefined;
  return site.call<T>(name, args);
}
