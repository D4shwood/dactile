// The Rust side's commands (src-tauri/src/lib.rs), typed.
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

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
  depth: number;
  book: boolean;
}

export interface TemplateCard {
  index: number;
  name: string;
  about: string;
  kind: "blank" | "built-in" | "own";
}

/// How the app and its page look.
export interface Look {
  theme: string;
  /// The surprise theme's hue and brightness, as `newSurprise` makes them.
  surprise: string;
  accent: string;
  page_font: string;
  page_size: number;
  line_spacing: number;
  page_width: number;
  page_zoom: number;
  animations: boolean;
}

export interface Settings extends Look {
  plays_dir: string;
  author: string;
  lang: "tr" | "en";
  ribbon_folded: boolean;
  viewer: Viewer;
}

/// A box of the PDF for a line: points from the page's top left.
export interface SyncPlace {
  page: number;
  x: number;
  y: number;
  width: number;
  height: number;
}

/// What the PDF viewer remembers (core's `config::Viewer`).
export interface Viewer {
  fit: "" | "width" | "text" | "window";
  zoom: number;
  continuous: boolean;
  columns: number;
  rows: number;
  single_step: boolean;
  offset: number;
  left: string;
  right: string;
  toolbar: boolean;
  invert: boolean;
  gray: boolean;
  lens_round: boolean;
  lens_size: number;
  lens_shadow: boolean;
  lens_border: boolean;
  follow_cursor: boolean;
  follow_scroll: boolean;
}

export interface Folder {
  path: string;
  shown: string;
  exists: boolean;
  temporary: boolean;
}

export interface Home {
  plays: PlayCard[];
  templates: TemplateCard[];
  settings: Settings;
  folder: Folder;
  /// What went wrong with the settings file.
  notice: string | null;
}

export interface Spot {
  path: string;
  /// `~/…`.
  shown: string;
  modified: number | null;
}

export interface Files {
  folder: Spot;
  pdf: Spot | null;
  copies: (Spot & { pdfs: number }) | null;
  table: Spot | null;
  drafts: (Spot & { title: string; draft: string | null })[];
  temporary: boolean;
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

export type CastRow =
  | { kind: "person"; name: string }
  | { kind: "group"; name: string; members: string[] };

export interface Cover {
  from: number;
  to: number;
  title: string;
  author: string;
  cast: CastRow[];
}

export interface View {
  cover: Cover | null;
  cast: Person[];
  blocks: Block[];
  tail: { start: number; end: number } | null;
  scene_align: string;
}

export interface Card {
  act: boolean;
  hidden: boolean;
  label: string;
  title: string;
  lines: number;
  at: number;
  end: number;
  places: number[];
}

export interface Change {
  from: number;
  to: number;
  insert: string;
  at: number;
}

export interface NoteCard {
  i: number;
  state: "open" | "done" | "hidden";
  archived: boolean;
  author: string;
  text: string;
  group: string;
  line: number | null;
  replies: [string, string][];
  from: number | null;
  to: number | null;
  gone: boolean;
  fixable: boolean;
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
  notes: (text: string) => invoke<NoteCard[]>("notes_list", { text }),
  noteAdd: (text: string, at: number, author: string, body: string) =>
    invoke<NoteCard[]>("note_add", { text, at, author, body }),
  noteSet: (text: string, i: number, state: string) => invoke<NoteCard[]>("note_set", { text, i, state }),
  noteReply: (text: string, i: number, author: string, reply: string) =>
    invoke<NoteCard[]>("note_reply", { text, i, author, reply }),
  notesPaste: (reply: string, author: string) => invoke<number>("notes_paste", { reply, author }),
  pickText: () => invoke<string | null>("pick_text"),
  noteFix: (text: string, i: number) => invoke<Change[]>("note_fix", { text, i }),
  noteAuthors: () => invoke<string[]>("note_authors"),
  startup: () => invoke<string | null>("startup"),
  pick: () => invoke<string | null>("pick"),
  pdf: (path: string) => invoke<ArrayBuffer>("pdf", { path }),
  fullScreen: (on: boolean) => invoke<void>("full_screen", { on }),
  pdfPrint: (path: string, first: number, last: number, copies: number) => invoke<void>("pdf_print", { path, first, last, copies }),
  openUrl: (url: string) => invoke<void>("open_url", { url }),
  syncView: (line: number, column: number) => invoke<SyncPlace[]>("sync_view", { line, column }),
  syncEdit: (pdf: string, page: number, x: number, y: number) => invoke<number>("sync_edit", { pdf, page, x, y }),
  misspelt: (text: string, from: number, to: number) => invoke<Checked>("misspelt", { text, from, to }),
  suggest: (word: string) => invoke<string[]>("suggest", { word }),
  learn: (word: string) => invoke<void>("learn", { word }),
  complete: (text: string, at: number, query: string, hash: boolean) =>
    invoke<Item[]>("complete", { text, at, query, hash }),
  verbs: (text: string) => invoke<string[]>("verbs", { text }),
  heading: (text: string, at: number, act: boolean, place: string, time: string) =>
    invoke<Change>("heading", { text, at, act, place, time }),
  cast: (text: string, byLines: boolean) => invoke<CastEntry[]>("cast", { text, byLines }),
  suggestAlias: (text: string, name: string) => invoke<string>("suggest_alias", { text, name }),
  characterAdd: (text: string, form: CharacterForm, at: number | null) =>
    invoke<Made>("character_add", { text, form, at }),
  renames: (text: string, old: string, newAlias: string) => invoke<number>("renames", { text, old, new: newAlias }),
  characterEdit: (text: string, old: string, form: CharacterForm) =>
    invoke<Made>("character_edit", { text, old, form }),
  characterRemove: (text: string, alias: string) => invoke<Made>("character_remove", { text, alias }),
  castOthers: (text: string) => invoke<string>("cast_others", { text }),
  castOthersSet: (text: string, words: string) =>
    invoke<Made | null>("cast_others_set", { text, words }),
  characterMerge: (text: string, from: string, into: string) =>
    invoke<Made>("character_merge", { text, from, into }),
  characterShift: (text: string, alias: string, up: boolean) =>
    invoke<Made | null>("character_shift", { text, alias, up }),
  summaries: (text: string) => invoke<Summary[]>("summaries", { text }),
  summaryHere: (text: string, at: number) =>
    invoke<{ at: number; title: string; text: string }>("summary_here", { text, at }),
  summarySet: (text: string, at: number, summary: string) =>
    invoke<Change | null>("summary_set", { text, at, summary }),
  sceneAlignNext: (text: string) => invoke<Made>("scene_align_next", { text }),
  sceneSplit: (text: string, at: number) => invoke<Made>("scene_split", { text, at }),
  sceneJoin: (text: string, at: number) => invoke<Made>("scene_join", { text, at }),
  fields: (text: string) => invoke<{ places: string[]; times: string[] }>("fields", { text }),
  castingRoles: (text: string) => invoke<Roles>("casting_roles", { text }),
  castingEdit: (edit: CastingEdit) => invoke<void>("casting_edit", { edit }),
  castingCsv: (text: string) => invoke<string>("casting_csv", { text }),
  castingPdf: (text: string) => invoke<string>("casting_pdf", { text }),
  castingCopies: (text: string, kind: CopyKind) => invoke<[string, number]>("casting_copies", { text, kind }),
  memo: (text: string, role: string, at: number) => invoke<Memo>("memo", { text, role, at }),
  pickImport: () => invoke<string | null>("pick_import"),
  importRead: (path: string, ocr = false) => invoke<Shown>("import_read", { path, ocr }),
  importStop: () => invoke<void>("import_stop"),
  /// Pages read, of how many, while a book is read from its images.
  onOcr: (f: (done: number, total: number) => void) =>
    listen<[number, number]>("ocr", (e) => f(e.payload[0], e.payload[1])),
  importEdit: (edit: ImportEdit) => invoke<Shown>("import_edit", { edit }),
  importDoc: () => invoke<{ text: string; page: string | null }[]>("import_doc"),
  importWrite: () => invoke<string>("import_write"),
  importVolume: () => invoke<PlayLine[]>("import_volume"),
  importWriteAll: (picked: number[], book: { title: string; front: boolean } | null) =>
    invoke<Written[]>("import_write_all", { picked, book }),
  bookPdf: (path: string) => invoke<string>("book_pdf", { path }),
  bookGather: (paths: string[], title: string) => invoke<string>("book_gather", { paths, title }),
  importDrop: () => invoke<void>("import_drop"),
  forkStart: (path: string, draft: string | null, today: string) => invoke<FormStart>("fork_start", { path, draft, today }),
  forkFile: (path: string, draft: string) => invoke<string>("fork_file", { path, draft }),
  copyStart: (title: string) => invoke<FormStart>("copy_start", { title }),
  titleFile: (title: string) => invoke<string>("title_file", { title }),
  renameStart: (path: string) => invoke<FormStart>("rename_start", { path }),
  playMake: (path: string, title: string | null, draft: string | null, file: string) => invoke<string>("play_make", { path, title, draft, file }),
  playRename: (path: string, title: string, file: string) => invoke<string>("play_rename", { path, title, file }),
  playDrafts: (path: string) => invoke<number>("play_drafts", { path }),
  playThrow: (path: string) => invoke<void>("play_throw", { path }),
  playRestore: () => invoke<string>("play_restore"),
  playSettings: () => invoke<PlaySettings>("play_settings"),
  playWpm: (wpm: number | null) => invoke<void>("play_wpm", { wpm }),
  playUnlearn: (word: string) => invoke<void>("play_unlearn", { word }),
  launch: (path: string) => invoke<void>("launch", { path }),
  reveal: (path: string) => invoke<void>("reveal", { path }),
  files: (path: string) => invoke<Files>("files", { path }),
};

export interface RoleRow {
  alias: string;
  name: string;
  actor: string | null;
  lines: number;
  words: number;
  time: string;
  scenes: number;
  share: number;
}

export interface SceneRow {
  name: string;
  place: string;
  at: number;
  time: string;
  lines: number;
  /// By role: 0 absent, 1 on stage, 2 speaks.
  here: number[];
  act: string | null;
}

export interface Roles {
  castings: string[];
  casting: number;
  wpm: number;
  roles: RoleRow[];
  scenes: SceneRow[];
  play: string;
  known: string[];
}

export type CastingEdit =
  | { kind: "actor"; alias: string; actor: string }
  | { kind: "new"; name: string }
  | { kind: "rename"; name: string }
  | { kind: "delete" }
  | { kind: "pick"; casting: number }
  | { kind: "wpm"; wpm: number };

export interface Cue {
  who: string | null;
  text: string;
  hint: string;
  ours: boolean;
  heading: boolean;
}

export interface Memo {
  who: string;
  cues: Cue[];
  at: number;
}

export interface Shown {
  file: string;
  title: string;
  author: string;
  people: { name: string; alias: string; lines: number; sample: string; not_character: boolean; merged: string | null }[];
  labels: string[];
  lines: number;
  directions: number;
  acts: number;
  scenes: number;
  plain: number;
  play: { kind: string; who: string | null; text: string }[];
  more: number;
  ocr: boolean;
  cast_text: string;
  silent: string[];
  missing: string[];
  plays: string[];
  picked: number | null;
}

export interface PlayLine {
  title: string;
  pages: [string, string] | null;
  lines: number;
  people: number;
  guess: boolean;
}

export interface Written {
  title: string;
  path: string | null;
  error: string | null;
}

export type ImportEdit =
  | { kind: "play"; i: number }
  | { kind: "info"; title: string; author: string }
  | { kind: "person"; i: number; name: string; alias: string }
  | { kind: "merge"; from: number; into: number }
  | { kind: "unmerge"; i: number }
  | { kind: "not-character"; i: number }
  | { kind: "mark"; from: number; to: number; page: string }
  | { kind: "scene"; from: number; to: number; on: boolean }
  | { kind: "start"; i: number }
  | { kind: "end"; i: number }
  | { kind: "add-play"; i: number; title: string }
  | { kind: "drop-play"; i: number }
  | { kind: "cast-page"; from: number; to: number }
  | { kind: "cast-list"; text: string };

export interface FormStart {
  title: string;
  draft: string;
  file: string;
}

export interface PlaySettings {
  wpm: number | null;
  default_wpm: number;
  words: string[];
  parent: string | null;
}

export type CopyKind = "full" | "rehearsal" | "light";

/// A scene's summary, under its heading (from `at` to the line's `end`).
export interface Summary {
  at: number;
  end: number;
  text: string;
}

/// A row of the cast panel (app/src-tauri/src/people.rs).
export interface CastEntry {
  group: boolean;
  alias: string;
  name: string;
  full: string;
  desc: string;
  star: boolean;
  kind: "cast" | "plain" | "hidden" | "member" | "group";
  lines: number;
  words: number;
}

export interface CharacterForm {
  name: string;
  alias: string;
  full: string;
  desc: string;
  star: boolean;
  in_table: boolean;
  hidden: boolean;
}

/// A change to the play from the cast panel: one undo step.
export interface Made {
  change: Change;
  cursor: number | null;
  message: string;
}

/// A suggestion after a backslash (app/src-tauri/src/complete.rs).
export interface Item {
  key: string;
  label: string;
  group: "scene" | "character" | "command" | "macro";
  action: "insert" | "scene" | "act" | "character";
  text: string;
  cursor: number;
}

export type Checked =
  | { kind: "loading" }
  | { kind: "missing"; why: string }
  | { kind: "words"; ranges: [number, number][] };
