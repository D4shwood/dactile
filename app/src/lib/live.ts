// A project's play, written live (in the browser): the page's copy of it,
// kept in step with its room in the gate (server/src/room.rs) over a
// WebSocket, in Yjs's protocol. The room says who writes and who holds the
// pen; the page asks it for the pen, to save, or to be alone for a moment.

import * as Y from "yjs";
import * as sync from "y-protocols/sync";
import * as aware from "y-protocols/awareness";
import * as encoding from "lib0/encoding";
import * as decoding from "lib0/decoding";

const SYNC = 0;
const AWARENESS = 1;
const QUERY = 3;
const ASK = 100;
const INFO = 101;
const SAVED = 102;
const BYE = 103;

/// What the room says to this page.
export interface RoomInfo {
  epoch: string;
  mode: "turns" | "together";
  role: "admin" | "writer" | "reader" | null;
  /// This page may write now.
  writes: boolean;
  /// Who holds the pen, in turns.
  pen: string | null;
  mine: boolean;
  /// The pen can be taken from here.
  take: boolean;
  /// Who has it open.
  people: string[];
  /// Someone moving scenes around, for a moment.
  alone: string | null;
  alone_mine: boolean;
  /// What was written is on the disk.
  saved: boolean;
  gone: boolean;
}

export type Link = "connecting" | "live" | "offline";

export interface Hooks {
  info: (i: RoomInfo) => void;
  link: (l: Link) => void;
  /// The room closed on this page, for good.
  bye: (why: string) => void;
  /// The play changed outside the room meanwhile: this copy is no longer
  /// its, and what was written here offline didn't get there.
  lost: (text: string) => void;
}

export class Live {
  doc = new Y.Doc();
  text = this.doc.getText("play");
  awareness = new aware.Awareness(this.doc);
  info: RoomInfo | null = null;
  link: Link = "connecting";
  /// Written here while cut off, not yet in the room.
  unsent = false;
  /// Resolved once the page has the room's play.
  synced: Promise<void>;
  private ready!: () => void;
  private ws: WebSocket | null = null;
  private wait = 500;
  private timer: ReturnType<typeof setTimeout> | undefined;
  private closed = false;
  private saving: ((r: { ok: boolean; error?: string }) => void)[] = [];
  private infoWaiting: ((i: RoomInfo) => void)[] = [];

  constructor(
    private path: string,
    private hooks: Hooks,
  ) {
    this.synced = new Promise((r) => (this.ready = r));
    this.doc.on("update", (update: Uint8Array, origin: unknown) => {
      if (origin === this) return;
      if (this.ws?.readyState !== WebSocket.OPEN || !this.joined) {
        this.unsent = true;
        return;
      }
      const e = encoding.createEncoder();
      encoding.writeVarUint(e, SYNC);
      sync.writeUpdate(e, update);
      this.send(encoding.toUint8Array(e));
    });
    this.awareness.on("update", ({ added, updated, removed }: { added: number[]; updated: number[]; removed: number[] }, origin: unknown) => {
      if (origin === this) return;
      const changed = added.concat(updated, removed);
      const e = encoding.createEncoder();
      encoding.writeVarUint(e, AWARENESS);
      encoding.writeVarUint8Array(e, aware.encodeAwarenessUpdate(this.awareness, changed));
      this.send(encoding.toUint8Array(e));
    });
    this.connect();
  }

  /// The room has told this page who it is, and the epoch matched.
  private joined = false;

  private send(b: Uint8Array) {
    if (this.ws?.readyState === WebSocket.OPEN) this.ws.send(b);
  }

  private setLink(l: Link) {
    this.link = l;
    this.hooks.link(l);
  }

  private connect() {
    if (this.closed) return;
    this.joined = false;
    const proto = location.protocol === "https:" ? "wss:" : "ws:";
    const ws = new WebSocket(`${proto}//${location.host}/api/oda?path=${encodeURIComponent(this.path)}`);
    ws.binaryType = "arraybuffer";
    this.ws = ws;
    ws.onopen = () => (this.wait = 500);
    ws.onmessage = (m) => this.hear(new Uint8Array(m.data as ArrayBuffer));
    ws.onclose = () => {
      if (this.ws !== ws) return;
      this.ws = null;
      this.joined = false;
      // Others' cursors mean nothing while cut off.
      aware.removeAwarenessStates(
        this.awareness,
        [...this.awareness.getStates().keys()].filter((c) => c !== this.doc.clientID),
        this,
      );
      if (this.closed) return;
      this.setLink("offline");
      this.timer = setTimeout(() => this.connect(), this.wait);
      this.wait = Math.min(this.wait * 2, 5000);
    };
  }

  private hear(data: Uint8Array) {
    const d = decoding.createDecoder(data);
    const kind = decoding.readVarUint(d);
    switch (kind) {
      case SYNC: {
        const e = encoding.createEncoder();
        encoding.writeVarUint(e, SYNC);
        const was = sync.readSyncMessage(d, e, this.doc, this);
        if (encoding.length(e) > 1) this.send(encoding.toUint8Array(e));
        if (was === sync.messageYjsSyncStep2) {
          this.unsent = false;
          this.setLink("live");
          this.ready();
        }
        break;
      }
      case AWARENESS:
        aware.applyAwarenessUpdate(this.awareness, decoding.readVarUint8Array(d), this);
        break;
      case QUERY: {
        const e = encoding.createEncoder();
        encoding.writeVarUint(e, AWARENESS);
        encoding.writeVarUint8Array(e, aware.encodeAwarenessUpdate(this.awareness, [this.doc.clientID]));
        this.send(encoding.toUint8Array(e));
        break;
      }
      case INFO: {
        const info = JSON.parse(new TextDecoder().decode(decoding.readVarUint8Array(d))) as RoomInfo;
        if (!this.joined && !this.join(info)) return;
        this.info = info;
        this.hooks.info(info);
        for (const w of this.infoWaiting.splice(0)) w(info);
        break;
      }
      case SAVED: {
        const r = JSON.parse(new TextDecoder().decode(decoding.readVarUint8Array(d)));
        this.saving.shift()?.(r);
        break;
      }
      case BYE: {
        const r = JSON.parse(new TextDecoder().decode(decoding.readVarUint8Array(d)));
        this.destroy();
        this.hooks.bye(r.why ?? "");
        break;
      }
    }
  }

  /// The room's first word on this connection: the same play as this
  /// page's, or one started anew while it was away.
  private join(info: RoomInfo): boolean {
    if (this.info && this.info.epoch !== info.epoch) {
      // Merging two histories would write everything twice.
      const text = this.text.toString();
      this.destroy();
      this.hooks.lost(this.unsent ? text : "");
      return false;
    }
    this.joined = true;
    const e = encoding.createEncoder();
    encoding.writeVarUint(e, SYNC);
    sync.writeSyncStep1(e, this.doc);
    this.send(encoding.toUint8Array(e));
    if (this.awareness.getLocalState()) {
      const a = encoding.createEncoder();
      encoding.writeVarUint(a, AWARENESS);
      encoding.writeVarUint8Array(a, aware.encodeAwarenessUpdate(this.awareness, [this.doc.clientID]));
      this.send(encoding.toUint8Array(a));
    }
    return true;
  }

  private ask(what: string) {
    const e = encoding.createEncoder();
    encoding.writeVarUint(e, ASK);
    encoding.writeVarUint8Array(e, new TextEncoder().encode(JSON.stringify({ do: what })));
    this.send(encoding.toUint8Array(e));
  }

  takePen() {
    this.ask("pen");
  }

  dropPen() {
    this.ask("drop");
  }

  /// The play onto the disk now.
  save(): Promise<{ ok: boolean; error?: string }> {
    if (this.ws?.readyState !== WebSocket.OPEN) return Promise.resolve({ ok: false, error: "offline" });
    const r = new Promise<{ ok: boolean; error?: string }>((res) => this.saving.push(res));
    this.ask("save");
    return r;
  }

  /// A moment with the play to oneself, to move scenes around: the
  /// others' editors wait. False if the room didn't give it.
  async alone(): Promise<boolean> {
    if (this.info?.mode !== "together" || this.info.people.length < 2) return true;
    const got = new Promise<RoomInfo>((res) => this.infoWaiting.push(res));
    this.ask("alone");
    const t = new Promise<null>((res) => setTimeout(() => res(null), 1500));
    const i = await Promise.race([got, t]);
    return !!i?.alone_mine;
  }

  done() {
    if (this.info?.alone_mine) this.ask("done");
  }

  destroy() {
    this.closed = true;
    clearTimeout(this.timer);
    aware.removeAwarenessStates(this.awareness, [this.doc.clientID], this);
    const ws = this.ws;
    this.ws = null;
    ws?.close();
    for (const s of this.saving.splice(0)) s({ ok: false, error: "closed" });
  }
}
