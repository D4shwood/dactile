// The Rust side's commands (src-tauri/src/lib.rs), typed.
import { invoke } from "@tauri-apps/api/core";

export interface PlayCard {
  path: string;
  title: string;
  author: string;
  acts: number;
  scenes: number;
  minutes: number;
  characters: number;
  words: number;
  draft: string | null;
  modified: number | null;
  elsewhere: boolean;
}

export interface TemplateCard {
  index: number;
  name: string;
  about: string;
  kind: "blank" | "built-in" | "own";
}

export interface Settings {
  plays_dir: string;
  author: string;
  lang: "tr" | "en";
  theme: string;
  page_zoom: number;
  ribbon_folded: boolean;
}

export interface Home {
  plays: PlayCard[];
  templates: TemplateCard[];
  settings: Settings;
}

export interface Preview {
  title: string;
  author: string;
  cast: string[];
  acts: [string, string[]][];
}

export interface Opened {
  path: string;
  name: string;
  text: string;
  read_only: boolean;
  holder: string | null;
}

export type Saved =
  | { kind: "saved" }
  | { kind: "read-only" }
  | { kind: "changed-elsewhere"; text: string };

export interface Mark {
  from: number;
  to: number;
  cls: string;
  label: string | null;
}

export interface Block {
  from: number;
  to: number;
  cls: string;
  marks: Mark[];
  alias: string | null;
}

export interface Person {
  alias: string;
  name: string;
}

export interface Cover {
  from: number;
  to: number;
  title: string;
  author: string;
  cast: string[];
}

export interface View {
  cover: Cover | null;
  cast: Person[];
  blocks: Block[];
  tail: { start: number; end: number } | null;
}

export interface Card {
  act: boolean;
  hidden: boolean;
  label: string;
  title: string;
  lines: number;
  at: number;
  places: number[];
}

export interface Change {
  from: number;
  to: number;
  insert: string;
  at: number;
}

export type BuildResult =
  | { kind: "built"; pdf: string; pages: number | null; seconds: number }
  | { kind: "failed"; line: number | null; message: string };

export const api = {
  home: () => invoke<Home>("home"),
  settings: () => invoke<Settings>("settings"),
  set: (key: string, value: unknown) => invoke<void>("set_setting", { key, value }),
  preview: (path: string) => invoke<Preview>("preview", { path }),
  create: (template: number, title: string) => invoke<string>("create", { template, title }),
  example: () => invoke<string>("example"),
  open: (path: string) => invoke<Opened>("open", { path }),
  close: () => invoke<void>("close"),
  save: (text: string, force = false) => invoke<Saved>("save", { text, force }),
  saveTemplate: (text: string, name: string) => invoke<string>("save_template", { text, name }),
  view: (text: string) => invoke<View>("view", { text }),
  board: (text: string) => invoke<Card[]>("board", { text }),
  relocate: (text: string, card: number, to: number) =>
    invoke<Change | null>("relocate", { text, card, to }),
  build: () => invoke<BuildResult>("build"),
  startup: () => invoke<string | null>("startup"),
  pick: () => invoke<string | null>("pick"),
  pdf: (path: string) => invoke<ArrayBuffer>("pdf", { path }),
};
