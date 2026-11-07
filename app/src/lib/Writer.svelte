<script lang="ts">
  import { onDestroy, onMount, tick, untrack } from "svelte";
  import { getVersion } from "@tauri-apps/api/app";
  import { EditorState } from "@codemirror/state";
  import { EditorView } from "@codemirror/view";
  import { redo, selectAll, undo } from "@codemirror/commands";
  import { openSearchPanel } from "@codemirror/search";
  import { api, type BuildResult, type Card, type CastEntry, type CharacterForm, type Files, type Look, type Made, type Memo, type Summary, type NoteCard, type Opened, type PlaySettings, type Preview, type Settings } from "./api";
  import { focus, setCards, setDim, setFocus } from "./focus";
  import { blockAt, currentView, extensions, insertSnippet, typeBackslash, coverClicked, openCover, pickNote, wrapSelection, setNotes, setSource, setView } from "./editor";
  import { BACKSTAGE, type Act } from "./ribbon";
  import { closeMenu, nextWord, recheck, showMenu, spelling } from "./spelling";
  import { completion, openMenu } from "./completion";
  import { kept } from "./session";
  import { caretAt, mouse } from "./mouse";
  import HeadingForm from "./HeadingForm.svelte";
  import CoverForm from "./CoverForm.svelte";
  import { findLang } from "./find";
  import Cast from "./Cast.svelte";
  import Casting from "./Casting.svelte";
  import Memorize from "./Memorize.svelte";
  import SummaryForm from "./SummaryForm.svelte";
  import { setSummaries, summaries } from "./summaries";
  import CharacterFormView from "./CharacterForm.svelte";
  import Ribbon from "./Ribbon.svelte";
  import Keys from "./Keys.svelte";
  import Guide from "./Guide.svelte";
  import Pdf from "./Pdf.svelte";
  import Board from "./Board.svelte";
  import Notes from "./Notes.svelte";
  import FileList from "./FileList.svelte";
  import LookForm from "./LookForm.svelte";
  import { applyLook, installedFonts, pickLook } from "./look";
  import Temporary from "./Temporary.svelte";

  let {
    opened,
    settings,
    tr,
    close,
  }: { opened: Opened; settings: Settings; tr: boolean; close: (next?: string) => void } = $props();

  const THEMES = ["paper", "sepia", "night", "contrast", "dark"];

  let host: HTMLDivElement;
  let view: EditorView;
  let ribbon: Ribbon | undefined = $state();
  let version = $state("");
  let dirty = $state(false);
  // Taken once: from here on the writer keeps them, and saves them.
  const first = untrack(() => settings);
  let folded = $state(first.ribbon_folded);
  let ribbonHeight = $state(first.ribbon_height || 0);
  let pdfWidth = $state(first.pdf_width || 0);
  /// While the PDF's edge is dragged.
  let pdfDrag = $state(0);
  const pdfShown = $derived(pdfDrag || pdfWidth);
  let coverLast = $state(first.cover_last ?? {});
  let coverForm = $state<"details" | "look" | null>(null);
  const openCoverForm = (tab: "details" | "look") => {
    board = false;
    coverForm = tab;
  };
  coverClicked.run = () => openCoverForm("details");
  findLang.tr = untrack(() => tr);

  function dragPdf(e: PointerEvent) {
    const edge = e.currentTarget as HTMLElement;
    const body = edge.parentElement!;
    edge.setPointerCapture(e.pointerId);
    const move = (m: PointerEvent) => {
      const box = body.getBoundingClientRect();
      const most = Math.max(320, box.width - 360);
      pdfDrag = Math.round(Math.min(most, Math.max(320, box.right - m.clientX)));
    };
    const up = () => {
      edge.removeEventListener("pointermove", move);
      edge.removeEventListener("pointerup", up);
      if (pdfDrag) {
        pdfWidth = pdfDrag;
        api.set("pdf_width", pdfWidth);
      }
      pdfDrag = 0;
      requestAnimationFrame(() => view.requestMeasure());
    };
    edge.addEventListener("pointermove", move);
    edge.addEventListener("pointerup", up);
    e.preventDefault();
  }

  async function writeCover(f: import("./api").CoverForm) {
    coverForm = null;
    const kept = Object.fromEntries(
      ["editedby", "copyrightnotice", "address", "email", "telephone"]
        .map((k) => [k, (f.fields[k] ?? "").trim()])
        .filter(([, v]) => v),
    );
    if (Object.keys(kept).length) {
      coverLast = { ...coverLast, ...kept };
      api.set("cover_last", coverLast);
    }
    await castCall(() => api.coverWrite(view.state.doc.toString(), f));
    view.focus();
  }
  let zoom = $state(Math.min(300, Math.max(50, first.page_zoom || 100)));
  let theme = $state(first.theme || "paper");
  /// The rest of the look; the page's zoom and theme have keys of their own.
  let look = $state(pickLook(first));
  let playFont = $state("");
  let source = $state(false);
  /// Where the play's class option puts its scene headings.
  let sceneAlign = $state("center");
  let typewriter = $state(false);
  /// What stands beside the page.
  let side = $state<"" | "pdf" | "notes" | "cast" | "files" | "look" | "guide">("");
  let pdf: Pdf | undefined = $state();
  let notesPanel: ReturnType<typeof Notes> | undefined = $state();
  let files = $state<Files | null>(null);
  /// The temporary folder's warning, put away for this play.
  let tempSeen = $state(false);
  let castRows = $state<CastEntry[]>([]);
  let castOthers = $state("");
  let sums = $state<Summary[]>([]);
  /// Alt+E: the scene being summed up.
  let summaryForm = $state<{ at: number; title: string; text: string } | null>(null);
  let byLines = $state(false);
  /// A character's form: who (null for a new one), and where its first
  /// line goes when it came from the menu.
  let characterForm = $state<{ old: string | null; start: CharacterForm; at: number | null; text: string } | null>(null);
  /// The page as a sheet on a desk (F7), or the text across the window.
  let sheet = $state(true);
  let noteCards = $state<NoteCard[]>([]);
  let picked = $state<number | null>(null);
  let authors = $state<string[]>([]);
  let draft = $state<string | null>(null);
  let pasting = $state<string | null>(null);
  let me = $state(first.author);
  let building = $state(false);
  let built = $state<BuildResult | null>(null);
  let board = $state(false);
  /// F4 and F8.
  let focused = $state(false);
  /// F9.
  let casting = $state(false);
  let memo = $state<Memo | null>(null);
  let dimmed = $state(false);
  let cards = $state<Card[]>([]);
  let backstage = $state(false);
  let page = $state("info");
  let info = $state<Preview | null>(null);
  let templateName = $state("");
  let message = $state<{ text: string; warn: boolean } | null>(null);
  let help = $state<"" | "keys" | "about">("");
  /// A new scene's or act's form, from the menu: where it goes.
  let headingForm = $state<{ act: boolean; at: number; places: string[]; times: string[] } | null>(null);
  let where = $state("");

  const l = (t: string, e: string) => (tr ? t : e);

  function say(text: string, warn = false) {
    message = { text, warn };
    const m = message;
    setTimeout(() => {
      if (message === m) message = null;
    }, 4000);
  }

  // ------------------------------------------------------------ the view

  let viewTimer: ReturnType<typeof setTimeout> | undefined;
  let saveTimer: ReturnType<typeof setTimeout> | undefined;

  function refresh(now = false) {
    clearTimeout(viewTimer);
    viewTimer = setTimeout(
      async () => {
        const doc = view.state.doc;
        const v = await api.view(doc.toString());
        // Typed on meanwhile: the next one is on its way.
        sceneAlign = v.scene_align;
        if (playFont !== v.play_font) {
          playFont = v.play_font;
          applyLook(shownLook());
          requestAnimationFrame(() => view.requestMeasure());
        }
        if (view.state.doc === doc) view.dispatch({ effects: setView.of(v) });
        if (board && view.state.doc === doc) cards = await api.board(doc.toString());
        if (focused && view.state.doc === doc) await refocus();
        if (side === "notes" && view.state.doc === doc) await refreshNotes();
        if (side === "cast" && view.state.doc === doc) await refreshCast();
        if (view.state.doc === doc) await refreshSummaries();
        locate();
      },
      now ? 0 : 140,
    );
  }

  /// "Sahne 2 — İç, Değirmen" for the status bar.
  function locate() {
    const v = currentView(view.state);
    if (!v) return;
    const head = view.state.selection.main.head;
    let act = "";
    let scene = "";
    for (const b of v.blocks) {
      if (b.from > head) break;
      const label = b.marks[0]?.label?.split("\n")[0] ?? "";
      if (b.cls === "act") (act = label), (scene = "");
      if (b.cls === "scene") scene = label;
    }
    where = [act, scene].filter(Boolean).join(" · ");
  }

  // ------------------------------------------------------------ my files

  async function loadFiles() {
    files = await api.files(opened.path).catch((e) => (say(String(e), true), files));
  }

  // What was just built shows in the open panel.
  $effect(() => {
    if (side === "files" && !building && !casting) void loadFiles();
  });

  // ------------------------------------------------------------ saving

  async function save(force = false): Promise<boolean> {
    if (opened.read_only) {
      say(l("Bu oyun salt okunur: başka bir Dactile'da açık.", "Read only: open in another Dactile."), true);
      return false;
    }
    clearTimeout(saveTimer);
    const text = view.state.doc.toString();
    try {
      const r = await api.save(text, force);
      if (r.kind === "saved") {
        if (view.state.doc.toString() === text) dirty = false;
        return true;
      }
      if (r.kind === "changed-elsewhere") {
        const keep = confirm(
          l(
            "Oyun bu arada başka bir yerde değişmiş. Seninkini yazayım mı? (Öbürü geçmişte kalır.)",
            "The play changed elsewhere meanwhile. Write yours? (The other stays in the history.)",
          ),
        );
        return keep ? save(true) : false;
      }
    } catch (e) {
      say(String(e), true);
    }
    return false;
  }

  async function build() {
    if (building) return;
    if (!(await save()) && !opened.read_only) return;
    side = "pdf";
    showNotes();
    keepCursor();
    building = true;
    try {
      built = await api.build();
      if (built.kind === "failed") say(l("PDF çıkmadı.", "No PDF."), true);
    } catch (e) {
      say(String(e), true);
    } finally {
      building = false;
    }
  }

  /// The page's width changed (the PDF panel): the cursor stays in sight.
  function keepCursor() {
    const head = view.state.selection.main.head;
    requestAnimationFrame(() => view.dispatch({ effects: EditorView.scrollIntoView(head, { y: "center" }) }));
  }

  function goto(line: number, quiet = false) {
    const n = Math.min(Math.max(line, 1), view.state.doc.lines);
    const pos = view.state.doc.line(n).from;
    board = false;
    view.dispatch({ selection: { anchor: pos }, scrollIntoView: true });
    if (!quiet) view.focus();
  }

  /// The place of a line in the PDF, shown there.
  async function showInPdf(pos = view.state.selection.main.head) {
    if (built?.kind !== "built") return say(l("Önce F5 ile derle.", "Build with F5 first."));
    const line = view.state.doc.lineAt(pos);
    if (side !== "pdf") {
      side = "pdf";
      showNotes();
      keepCursor();
      await tick();
    }
    await pdf?.reveal(line.number, pos - line.from);
  }

  /// Memorizing a role from its next line on; Esc goes back to where it
  /// was opened from.
  async function openMemo(alias: string) {
    memo = await api.memo(view.state.doc.toString(), alias, view.state.selection.main.head);
  }

  /// The ribbon's button: the role of the line the cursor is on.
  function memorizeHere() {
    const alias = blockAt(currentView(view.state), view.state.selection.main.head)?.alias;
    if (alias) return void openMemo(alias);
    say(l("İmleci bir rolün satırına götür ya da F9'dan bir rol seç.", "Put the cursor on a role's line, or pick a role on F9."));
    casting = true;
  }

  function at(pos: number) {
    board = false;
    casting = false;
    view.dispatch({ selection: { anchor: pos }, effects: EditorView.scrollIntoView(pos, { y: "start", yMargin: 60 }) });
    view.focus();
  }

  // ------------------------------------------------------------ summaries

  async function refreshSummaries() {
    const doc = view.state.doc;
    const s = await api.summaries(doc.toString()).catch(() => [] as Summary[]);
    if (view.state.doc !== doc) return;
    sums = s;
    view.dispatch({ effects: setSummaries.of(s) });
  }

  /// Alt+E: the summary of the cursor's scene.
  async function summaryHere() {
    if (opened.read_only) return;
    try {
      summaryForm = await api.summaryHere(view.state.doc.toString(), view.state.selection.main.head);
    } catch (e) {
      say(String(e));
    }
  }

  async function summaryDone(text: string) {
    const f = summaryForm!;
    summaryForm = null;
    try {
      const c = await api.summarySet(view.state.doc.toString(), f.at, text);
      // The heading's id (%@k3), when it had none: hidden on the page.
      if (c) view.dispatch({ changes: { from: c.from, to: c.to, insert: c.insert }, userEvent: "input.summary" });
      await refreshSummaries();
      say(text.trim() ? l("Özet kaydedildi (PDF'e girmez).", "Summary kept (never printed).") : l("Özet silindi.", "Summary removed."));
    } catch (e) {
      say(String(e), true);
    }
    view.focus();
  }

  // ------------------------------------------------------------ the cast

  async function refreshCast() {
    castRows = await api.cast(view.state.doc.toString(), byLines);
    castOthers = await api.castOthers(view.state.doc.toString());
  }

  async function openCast(on = side !== "cast") {
    side = on ? "cast" : "";
    showNotes();
    keepCursor();
    if (on) await refreshCast();
  }

  $effect(() => {
    byLines;
    if (untrack(() => side) === "cast") void refreshCast();
  });

  /// A change from the cast panel or form: one undo step.
  function applyMade(m: Made) {
    const c = m.change;
    view.dispatch({
      changes: { from: c.from, to: c.to, insert: c.insert },
      selection: m.cursor !== null ? { anchor: m.cursor } : undefined,
      effects: m.cursor !== null ? EditorView.scrollIntoView(m.cursor, { y: "center" }) : [],
      userEvent: "input.cast",
    });
    if (m.message) say(m.message);
    if (side === "cast") void refreshCast();
  }

  async function castCall(f: () => Promise<Made | null>) {
    try {
      const m = await f();
      if (m) applyMade(m);
    } catch (e) {
      say(String(e), true);
    }
  }

  function newCharacter(at: number | null = null) {
    if (opened.read_only) return;
    characterForm = {
      old: null,
      at,
      text: view.state.doc.toString(),
      start: { name: "", alias: "", full: "", desc: "", star: false, in_table: true, hidden: false },
    };
  }

  function editCharacter(r: CastEntry) {
    if (opened.read_only) return;
    characterForm = {
      old: r.alias,
      at: null,
      text: view.state.doc.toString(),
      start: {
        name: r.name,
        alias: r.alias,
        full: r.full,
        desc: r.desc,
        star: r.star,
        in_table: r.kind === "cast" || r.kind === "member",
        hidden: r.kind === "hidden",
      },
    };
  }

  /// The form's answer: null when done, else why not.
  async function characterDone(f: CharacterForm): Promise<string | null> {
    const cf = characterForm!;
    const text = view.state.doc.toString();
    try {
      const m = cf.old === null ? await api.characterAdd(text, f, cf.at) : await api.characterEdit(text, cf.old, f);
      characterForm = null;
      applyMade(m);
      view.focus();
      return null;
    } catch (e) {
      return String(e);
    }
  }

  /// A character's first line, picked.
  function firstLine(alias: string) {
    const b = currentView(view.state)?.blocks.find((b) => b.cls === "line" && b.alias === alias);
    if (!b) return say(l(`\${alias} henüz konuşmuyor.`, `\${alias} hasn't spoken yet.`));
    view.dispatch({ selection: { anchor: b.to }, effects: EditorView.scrollIntoView(b.from, { y: "center" }) });
    view.focus();
  }

  // ------------------------------------------------------------ notes

  async function refreshNotes() {
    noteCards = await api.notes(view.state.doc.toString());
    showNotes();
  }

  /// The notes' places underlined while the panel is open.
  function showNotes() {
    const on = side === "notes";
    const marks = on
      ? noteCards
          .filter((c) => c.from !== null && c.to !== null && !c.archived && c.state !== "hidden")
          .map((c) => ({ from: c.from!, to: c.to!, i: c.i, done: c.state === "done" }))
      : [];
    view.dispatch({ effects: [setNotes.of(marks), pickNote.of(on ? picked : null)] });
  }

  async function openNotes(on = side !== "notes") {
    side = on ? "notes" : "";
    keepCursor();
    if (on) {
      authors = await api.noteAuthors();
      await refreshNotes();
    } else showNotes();
  }

  /// A note on the selection, or on the word at the cursor.
  async function noteHere() {
    const st = view.state;
    const sel = st.selection.main;
    let quote = "";
    if (!sel.empty) {
      const t = st.sliceDoc(sel.from, sel.to).trim();
      if (!t.includes("\n") && t.length <= 120) quote = t;
    } else {
      const line = st.doc.lineAt(sel.head);
      const at = sel.head - line.from;
      const word = /[\p{L}\p{N}'’]/u;
      let a = at;
      let b = at;
      while (a > 0 && word.test(line.text[a - 1])) a--;
      while (b < line.text.length && word.test(line.text[b])) b++;
      if (b > a && line.text[a - 1] !== "\\") quote = line.text.slice(a, b);
    }
    draft = quote ? `“${quote}” ` : "";
    if (side !== "notes") await openNotes(true);
  }

  function pickCard(i: number) {
    picked = i;
    const c = noteCards.find((n) => n.i === i);
    view.dispatch({ effects: pickNote.of(i) });
    if (c?.from != null) {
      view.dispatch({
        selection: { anchor: c.from, head: c.to ?? c.from },
        effects: EditorView.scrollIntoView(c.from, { y: "center" }),
      });
    }
  }

  async function noteSet(i: number, state: string) {
    noteCards = await api.noteSet(view.state.doc.toString(), i, state).catch((e) => (say(String(e), true), noteCards));
    if (state === "delete") picked = null;
    showNotes();
  }

  async function noteAdd(author: string, body: string) {
    try {
      noteCards = await api.noteAdd(view.state.doc.toString(), view.state.selection.main.head, author, body);
      draft = null;
      picked = noteCards[noteCards.length - 1]?.i ?? null;
      authors = await api.noteAuthors();
      showNotes();
    } catch (e) {
      say(String(e), true);
    }
  }

  async function noteReply(i: number, author: string, text: string) {
    noteCards = await api.noteReply(view.state.doc.toString(), i, author, text).catch((e) => (say(String(e), true), noteCards));
  }

  async function noteFix(i: number) {
    try {
      const changes = await api.noteFix(view.state.doc.toString(), i);
      // One step: one Ctrl+Z puts the words back.
      view.dispatch({ changes: changes.map((c) => ({ from: c.from, to: c.to, insert: c.insert })), userEvent: "input" });
      say(l("Düzeltildi; not yapıldı sayıldı. Ctrl+Z geri alır.", "Fixed; the note is done. Ctrl+Z undoes it."));
      await refreshNotes();
    } catch (e) {
      say(String(e), true);
    }
  }

  async function noteFixMany(which: number[]) {
    await castCall(() => api.noteFixMany(view.state.doc.toString(), which));
    await refreshNotes();
  }

  async function notesPaste(text: string, author: string) {
    try {
      const n = await api.notesPaste(text, author);
      pasting = null;
      say(n ? l(`${n} not eklendi.`, `${n} notes added.`) : l("Numaralı madde bulunamadı (“139: …”).", "No numbered items found (“139: …”)."), !n);
      authors = await api.noteAuthors();
      await refreshNotes();
    } catch (e) {
      say(String(e), true);
    }
  }

  async function pasteNotes() {
    if (side !== "notes") await openNotes(true);
    pasting = await navigator.clipboard.readText().catch(() => "");
  }

  async function notesFromFile() {
    const t = await api.pickText();
    if (t === null) return;
    if (side !== "notes") await openNotes(true);
    pasting = t;
  }

  /// The scenes for F4, as the board sees them.
  async function refocus() {
    const doc = view.state.doc;
    const c = await api.board(doc.toString());
    if (focused && view.state.doc === doc) view.dispatch({ effects: [setCards.of({ cards: c, tr }), setFocus.of(true)] });
  }

  // ------------------------------------------------------------ the board

  async function openBoard() {
    cards = await api.board(view.state.doc.toString());
    board = !board;
  }

  async function move(card: number, to: number) {
    const c = await api.relocate(view.state.doc.toString(), card, to);
    if (!c) return;
    // One change, so one undo puts it back.
    view.dispatch({ changes: { from: c.from, to: c.to, insert: c.insert }, userEvent: "move" });
    refresh(true);
    say(l("Taşındı. Ctrl+Z geri alır.", "Moved. Ctrl+Z puts it back."));
  }

  // ------------------------------------------------------------ settings

  function setZoom(z: number) {
    zoom = Math.round(Math.min(300, Math.max(50, z)) / 10) * 10;
    api.set("page_zoom", zoom);
    requestAnimationFrame(() => view.requestMeasure());
  }

  /// The play's own font, when it has one, wins on the page.
  const shownLook = () => ({ ...look, page_font: playFont || look.page_font, theme, page_zoom: zoom });

  function setPlayFont(f: string) {
    if (opened.read_only) return;
    void castCall(() => api.playFontWrite(view.state.doc.toString(), f));
  }

  function setTheme(t: string) {
    theme = t;
    applyLook(shownLook());
    api.set("theme", t);
  }

  function setLook<K extends keyof Look>(key: K, value: Look[K]) {
    if (key === "theme") setTheme(value as string);
    else if (key === "page_zoom") setZoom(value as number);
    else {
      look[key] = value;
      applyLook(shownLook());
      api.set(key, value).catch((e) => say(String(e), true));
      requestAnimationFrame(() => view.requestMeasure());
    }
  }

  // ------------------------------------------------------------ actions

  async function clip(what: "copy" | "cut" | "paste") {
    view.focus();
    if (what === "paste") {
      try {
        const text = await navigator.clipboard.readText();
        view.dispatch(view.state.replaceSelection(text));
      } catch {
        document.execCommand("paste");
      }
    } else document.execCommand(what);
  }

  function key(id: string) {
    switch (id) {
      case "save":
        save().then((ok) => {
          if (!ok) return;
          if (files?.temporary) say(l("Kaydedildi, ama bu klasör geçici: bilgisayar yeniden başlayınca silinebilir.", "Saved, but this folder is temporary: it may be erased when the computer restarts."), true);
          else say(l("Kaydedildi.", "Saved."));
        });
        break;
      case "build":
        build();
        break;
      case "undo":
        undo(view);
        break;
      case "redo":
        redo(view);
        break;
      case "select-all":
        selectAll(view);
        break;
      case "copy":
      case "cut":
      case "paste":
        clip(id);
        break;
      case "find":
        openSearchPanel(view);
        break;
      case "spelling":
        spellNext();
        break;
      case "search":
        view.focus();
        if (!opened.read_only) openMenu(view, typeBackslash);
        break;
      case "commands":
        ribbon?.focusSearch();
        break;
      case "page":
        sheet = !sheet;
        say(sheet ? l("Sayfa: masanın üstünde bir kâğıt.", "Page: a sheet on a desk.") : l("Sayfa kapalı: metin pencere boyunca.", "Page off: the text across the window."));
        requestAnimationFrame(() => view.requestMeasure());
        break;
      case "source":
        source = !source;
        view.dispatch({ effects: setSource.of(source) });
        break;
      case "docx":
        api
          .docxExport(opened.path, view.state.doc.toString())
          .then((p) => say(l(`Word belgesi yazıldı: ${p}`, `Word document written: ${p}`)))
          .catch((e) => say(String(e), true));
        break;
      case "scene-align":
        if (!opened.read_only) void castCall(() => api.sceneAlignNext(view.state.doc.toString()));
        break;
      case "pdf-show":
        void showInPdf();
        break;
      case "pdf-panel":
        side = side === "pdf" ? "" : "pdf";
        // The notes' underlines go with their panel.
        showNotes();
        keepCursor();
        break;
      case "notes":
        openNotes();
        break;
      case "files":
      case "look":
        side = side === id ? "" : id;
        showNotes();
        keepCursor();
        break;
      case "cast":
        openCast();
        break;
      case "summary":
        summaryHere();
        break;
      case "focus":
        focused = !focused;
        if (focused) void refocus();
        else view.dispatch({ effects: setFocus.of(false) });
        say(focused ? l("Sahne odağı: yalnız bu sahne.", "Scene focus: this scene only.") : l("Sahne odağı kapalı.", "Scene focus off."));
        break;
      case "dim":
        dimmed = !dimmed;
        view.dispatch({ effects: setDim.of(dimmed) });
        say(dimmed ? l("Soldurma: yalnız imleçteki blok parlak.", "Dim: only the cursor's block lit.") : l("Soldurma kapalı.", "Dim off."));
        break;
      case "split":
      case "join":
        if (!opened.read_only) {
          const text = view.state.doc.toString();
          const at = view.state.selection.main.head;
          void castCall(() => (id === "split" ? api.sceneSplit(text, at) : api.sceneJoin(text, at)));
        }
        break;
      case "new-character":
        newCharacter();
        break;
      case "note-here":
        noteHere();
        break;
      case "notes-paste":
        pasteNotes();
        break;
      case "notes-file":
        notesFromFile();
        break;
      case "zoom-in":
        setZoom(zoom + 10);
        break;
      case "zoom-out":
        setZoom(zoom - 10);
        break;
      case "zoom-reset":
        setZoom(100);
        break;
      case "typewriter":
        typewriter = !typewriter;
        say(typewriter ? l("Daktilo: imleç ortada kalır.", "Typewriter: the cursor stays in the middle.") : l("Daktilo kapalı.", "Typewriter off."));
        break;
      case "theme":
        setTheme(THEMES[(THEMES.indexOf(theme) + 1) % THEMES.length]);
        break;
      case "fold":
        folded = !folded;
        api.set("ribbon_folded", folded);
        break;
      case "board":
        memo = null;
        openBoard();
        break;
      case "memorize":
        memorizeHere();
        break;
      case "casting":
        board = false;
        memo = null;
        casting = !casting;
        if (!casting) view.focus();
        break;
      case "cover":
        openCoverForm("details");
        break;
      case "cover-look":
        openCoverForm("look");
        break;
      case "cover-source":
        openCover(view);
        break;
      case "guide":
        side = side === "guide" ? "" : "guide";
        break;
      case "bold":
        wrapSelection(view, "\\textbf{", "}");
        break;
      case "italic":
        wrapSelection(view, "\\textit{", "}");
        break;
      case "underline":
        wrapSelection(view, "\\underline{", "}");
        break;
      case "keys":
        help = "keys";
        break;
      case "about":
        help = "about";
        break;
      case "info":
        openBackstage("info");
        break;
      case "save-template":
        openBackstage("template");
        break;
      case "play-settings":
        void openBackstage("settings").then(openPlaySettings);
        break;
      case "close":
        leave();
        break;
    }
    if (!["commands", "casting", "memorize", "find", "spelling", "search", "summary", "note-here", "notes-paste", "notes-file"].includes(id)) view.focus();
  }

  function run(a: Act) {
    if (a.do === "key") key(a.id);
    else if (a.do === "insert") {
      board = false;
      insertSnippet(view, a.snippet, a.block);
    }
  }

  async function openBackstage(p = "info") {
    backstage = true;
    page = p;
    info = await api.preview(opened.path).catch(() => null);
  }

  // The play's own settings, kept beside it.
  let playSettings = $state<PlaySettings | null>(null);

  async function openPlaySettings() {
    page = "settings";
    playSettings = await api.playSettings();
  }

  async function setWpm(wpm: number | null) {
    try {
      await api.playWpm(wpm);
    } catch (e) {
      say(String(e), true);
    }
    playSettings = await api.playSettings();
  }

  async function unlearn(word: string) {
    await api.playUnlearn(word);
    playSettings = await api.playSettings();
    recheck(view);
  }

  async function saveTemplate() {
    try {
      const path = await api.saveTemplate(view.state.doc.toString(), templateName);
      say(l(`Şablon kaydedildi: ${path}`, `Template saved: ${path}`));
      backstage = false;
      templateName = "";
    } catch (e) {
      say(String(e), true);
    }
  }

  /// Back to the start, or on to the play `next`.
  async function leave(next?: string) {
    if (dirty && !(await save())) {
      if (!confirm(l("Kaydedilemedi. Yine de kapatayım mı?", "Couldn't save. Close anyway?"))) return;
    }
    await api.close();
    close(next);
  }

  function onkey(e: KeyboardEvent) {
    const mod = e.ctrlKey || e.metaKey;
    const keys: Record<string, string> = {
      F1: mod ? "fold" : "keys",
      F2: "source",
      F3: "typewriter",
      F4: "focus",
      F5: "build",
      F6: "theme",
      F7: "page",
      F8: "dim",
      F9: "casting",
      F12: "pdf-panel",
    };
    let id = keys[e.key];
    if (mod && !e.altKey) {
      if (e.key === "=" || e.key === "+") id = "zoom-in";
      if (e.key === "-") id = "zoom-out";
      if (e.key === "0") id = "zoom-reset";
      if (e.key === "g") id = "board";
      if (e.key === "p") id = "commands";
      if (e.key === "w") id = "close";
      if (e.key === "n") id = "notes";
      if (e.key === "k") id = "search";
      if (e.key === "o") id = "cast";
      if (e.key === "'") id = "pdf-show";
      if (e.key === "N" && e.shiftKey) {
        e.preventDefault();
        notesPanel?.focusList();
        return;
      }
      if (e.key === "b") id = "bold";
      if (e.key === "i") id = "italic";
      if (e.key === "u") id = "underline";
    }
    if (e.altKey && !mod && /^[iıİI]$/.test(e.key)) id = "cover";
    if (e.altKey && !mod && (e.key === "p" || e.key === "P")) id = "cover-source";
    if (e.altKey && !mod && (e.key === "n" || e.key === "N")) id = "note-here";
    if (e.altKey && !mod && (e.key === "s" || e.key === "S")) id = "spelling";
    if (e.altKey && !mod && (e.key === "e" || e.key === "E")) id = "summary";
    if (e.altKey && !mod && (e.key === "b" || e.key === "B")) id = "split";
    if (e.altKey && !mod && (e.key === "j" || e.key === "J")) id = "join";
    if (e.key === "Escape") {
      if (help) return void (help = "");
      if (backstage) return void (backstage = false);
      if (board) return void (board = false);
      if (memo) return void ((memo = null), casting || view.focus());
      if (casting) return void ((casting = false), view.focus());
      if (draft !== null || pasting !== null) return void ((draft = null), (pasting = null));
    }
    if (id) {
      e.preventDefault();
      key(id);
    }
  }

  function onwheel(e: WheelEvent) {
    if (!e.ctrlKey) return;
    // Ctrl+wheel: the page grows, the window and the ribbon don't.
    e.preventDefault();
    setZoom(zoom + (e.deltaY < 0 ? 10 : -10));
  }

  /// A suggestion that is a form: a new scene or act (characters: soon).
  async function openForm(kind: "scene" | "act" | "character", at: number) {
    if (kind === "character") return newCharacter(at);
    const f = await api.fields(view.state.doc.toString());
    headingForm = { act: kind === "act", at, ...f };
  }

  async function writeHeading(a: string, b: string) {
    const f = headingForm!;
    headingForm = null;
    const c = await api.heading(view.state.doc.toString(), f.at, f.act, a, b);
    view.dispatch({
      changes: { from: c.from, to: c.to, insert: c.insert },
      selection: { anchor: c.at },
      scrollIntoView: true,
      userEvent: "input.complete",
    });
    view.focus();
  }

  /// Alt+S: the next misspelt word, picked, with its suggestions.
  function spellNext() {
    const w = nextWord(view.state, view.state.selection.main.head);
    if (!w) return say(l("Yazım hatası görünmüyor.", "No misspelt words in sight."));
    view.dispatch({
      selection: { anchor: w.from, head: w.to },
      effects: EditorView.scrollIntoView(w.from, { y: "center" }),
    });
    requestAnimationFrame(() => {
      const c = view.coordsAtPos(w.from);
      if (c) void showMenu(view, w, c.left, c.bottom + 4, tr, true);
    });
  }

  // ------------------------------------------------------------ life

  onMount(() => {
    applyLook(shownLook());
    void loadFiles();
    view = new EditorView({
      parent: host,
      state: EditorState.create({
        doc: kept.get(opened) ?? opened.text,
        extensions: [
          extensions({
            changed: () => {
              dirty = true;
              refresh();
              clearTimeout(saveTimer);
              // Saved on its own a few seconds after the last keystroke.
              saveTimer = setTimeout(() => dirty && save(), 4000);
            },
            save: () => key("save"),
          }),
          completion({
            tr: () => tr,
            form: (kind, at) => void openForm(kind, at),
          }),
          summaries,
          focus,
          mouse,
          spelling({
            tr: () => tr,
            on: () => true,
            missing: (why) => say(why, true),
          }),
          EditorState.readOnly.of(opened.read_only),
          // Ctrl+click: the place in the PDF.
          EditorView.domEventHandlers({
            mousedown(e, v) {
              if (e.button !== 0 || !(e.ctrlKey || e.metaKey) || e.shiftKey || e.altKey) return false;
              const pos = caretAt(v, e.clientX, e.clientY);
              if (pos === null) return false;
              e.preventDefault();
              void showInPdf(pos);
              return true;
            },
          }),
          EditorView.updateListener.of((u) => {
            if (u.selectionSet) {
              locate();
              if (view?.hasFocus) {
                const head = u.state.selection.main.head;
                const line = u.state.doc.lineAt(head);
                pdf?.cursorMoved(line.number, head - line.from);
              }
              if (typewriter) {
                const head = u.state.selection.main.head;
                requestAnimationFrame(() =>
                  view.dispatch({ effects: EditorView.scrollIntoView(head, { y: "center" }) }),
                );
              }
            }
          }),
        ],
      }),
    });
    refresh(true);
    void getVersion().then((v) => (version = v));
    // For trying the app out from a script (DACTILE_DEV_EVAL).
    if (import.meta.env.DEV) (window as any).__dactile = { view, key, run, move, openBoard, build, setZoom };
    // To the first line of dialogue, past the cover.
    api.view(opened.text).then((v) => {
      const first = v.blocks.find((b) => b.cls === "line" || b.cls === "scene");
      if (first) view.dispatch({ selection: { anchor: first.to }, scrollIntoView: true });
      view.focus();
    });
  });

  onDestroy(() => {
    if (view) kept.set(opened, view.state.doc.toString());
    closeMenu();
    clearTimeout(viewTimer);
    clearTimeout(saveTimer);
    view?.destroy();
  });

</script>

<svelte:window onkeydown={onkey} />

<div class="writer">
  <Ribbon
    font={playFont}
    fonts={installedFonts(playFont)}
    setFont={setPlayFont}
    bind:this={ribbon}
    {tr}
    {folded}
    {dirty}
    title={`${opened.name}${opened.read_only ? l(" (salt okunur)", " (read only)") : ""} — Dactile`}
    {run}
    file={() => openBackstage("info")}
    height={ribbonHeight}
    resized={(h) => {
      ribbonHeight = h;
      api.set("ribbon_height", h);
      requestAnimationFrame(() => view.requestMeasure());
    }}
  />

  {#if files?.temporary && !tempSeen}
    <Temporary
      {tr}
      shown={files.folder.shown}
      label={l("Nerede?", "Where is it?")}
      act={() => side === "files" || key("files")}
      dismiss={() => (tempSeen = true)}
    />
  {/if}

  <main class="body" class:split={side !== ""} style={side === "pdf" && pdfShown ? `--pdf-col:${pdfShown}px` : ""}>
    <div class="left">
      <div class="desk" class:gone={board || casting || memo} class:plain={!sheet} {onwheel}>
        <div class="page" class:source class:typewriter data-scene-align={sceneAlign} style={`--z: ${zoom / 100}`}>
          <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
          <div
            bind:this={host}
            onclick={(e) => {
              const n = (e.target as HTMLElement).closest("[data-note]") as HTMLElement | null;
              if (n && side === "notes") picked = Number(n.dataset.note);
              if (n && side === "notes") view.dispatch({ effects: pickNote.of(picked) });
            }}
          ></div>
        </div>
      </div>
      {#if board}
        <Board {tr} {cards} {move} open={at} close={() => (board = false)} summaries={Object.fromEntries(sums.map((s) => [s.at, s.text]))} />
      {:else if memo}
        <Memorize {tr} {memo} close={() => ((memo = null), casting || view.focus())} />
      {:else if casting}
        <Casting
          {tr}
          text={() => view.state.doc.toString()}
          readOnly={opened.read_only}
          go={at}
          memorize={(alias) => openMemo(alias)}
          {say}
          close={() => ((casting = false), view.focus())}
        />
      {/if}
    </div>
    {#if side === "pdf"}
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <div
        class="pdf-edge"
        style={`right:${pdfShown ? pdfShown + "px" : "max(320px, 40%)"}`}
        title={l("Sürükle; çift tık: varsayılan", "Drag; double click: its own width")}
        onpointerdown={dragPdf}
        ondblclick={() => {
          pdfWidth = 0;
          api.set("pdf_width", 0);
          requestAnimationFrame(() => view.requestMeasure());
        }}
      ></div>
      <Pdf bind:this={pdf} {tr} result={built} {building} close={() => ((side = ""), view.focus())} {goto} {say} />
    {:else if side === "files"}
      <aside class="files">
        <div class="head">
          <strong>{l("Dosyalarım", "My files")}</strong>
          <span class="grow"></span>
          <button title={l("Kapat", "Close")} onclick={() => key("files")}>✕</button>
        </div>
        {#if files}
          <div class="list"><FileList {tr} {files} open={leave} reload={loadFiles} /></div>
        {/if}
      </aside>
    {:else if side === "guide"}
      <Guide {tr} close={() => (side = "")} />
    {:else if side === "look"}
      <aside class="files">
        <div class="head">
          <strong>{l("Görünüş ayarları", "Appearance")}</strong>
          <span class="grow"></span>
          <button title={l("Kapat", "Close")} onclick={() => key("look")}>✕</button>
        </div>
        <div class="list"><LookForm {tr} look={{ ...look, theme, page_zoom: zoom }} set={setLook} /></div>
      </aside>
    {:else if side === "cast"}
      <Cast
        {tr}
        rows={castRows}
        others={castOthers}
        setOthers={(w) => castCall(() => api.castOthersSet(view.state.doc.toString(), w))}
        bind:byLines
        add={() => newCharacter()}
        edit={editCharacter}
        remove={(a) => castCall(() => api.characterRemove(view.state.doc.toString(), a))}
        merge={(a, b) => castCall(() => api.characterMerge(view.state.doc.toString(), a, b))}
        shift={(a, up) => castCall(() => api.characterShift(view.state.doc.toString(), a, up))}
        find={firstLine}
        close={() => openCast(false)}
      />
    {:else if side === "notes"}
      <Notes
        {tr}
        cards={noteCards}
        {picked}
        {authors}
        bind:draft
        bind:pasting
        bind:me
        pick={pickCard}
        set={noteSet}
        add={noteAdd}
        reply={noteReply}
        fix={noteFix}
        fixMany={noteFixMany}
        back={() => view.focus()}
        bind:this={notesPanel}
        paste={notesPaste}
        fromFile={notesFromFile}
        close={() => openNotes(false)}
      />
    {/if}
  </main>

  <footer class="status">
    <span>{where}</span>
    <span class="grow"></span>
    {#if message}<span class:warn={message.warn}>{message.text}</span>{/if}
    {#if opened.read_only}
      <span class="warn">{l(`Salt okunur: ${opened.holder ?? "başka bir Dactile"} açık tutuyor.`, `Read only: held by ${opened.holder ?? "another Dactile"}.`)}</span>
    {:else}
      <span class="muted">{dirty ? l("kaydedilmedi", "unsaved") : l("kaydedildi", "saved")}</span>
    {/if}
    <button class="zoom" title="Ctrl+0" onclick={() => setZoom(100)}>{zoom}%</button>
    <span class="muted">{theme}</span>
    <span class="muted">Alt: {l("şerit harfleri", "ribbon letters")}</span>
  </footer>

  {#if backstage}
    <div class="backstage">
      <nav>
        <button class="back" onclick={() => (backstage = false)}>← {l("Geri", "Back")}</button>
        {#each BACKSTAGE as e}
          <button
            class:on={(e.act.do === "key" && e.act.id === "info" && page === "info") ||
              (e.act.do === "key" && e.act.id === "play-settings" && page === "settings") ||
              (e.act.do === "key" && e.act.id === "save-template" && page === "template")}
            disabled={e.act.do === "soon"}
            onclick={() => {
              if (e.act.do !== "key") return;
              if (e.act.id === "info") page = "info";
              else if (e.act.id === "save-template") page = "template";
              else if (e.act.id === "play-settings") void openPlaySettings();
              else {
                backstage = false;
                key(e.act.id);
              }
            }}
          >
            {tr ? e.tr : e.en}
            {#if e.key}<small>{e.key}</small>{/if}
          </button>
        {/each}
      </nav>
      <section>
        {#if page === "info"}
          <h1>{info?.title ?? opened.name}</h1>
          <p class="muted">{info?.author}</p>
          <dl>
            <dt>{l("Dosya", "File")}</dt>
            <dd>{opened.path}</dd>
            <dt>{l("Kişiler", "Cast")}</dt>
            <dd>{info?.cast.join(" · ")}</dd>
          </dl>
          {#each info?.acts ?? [] as [act, scenes]}
            <h3>{act}</h3>
            <ul>
              {#each scenes as s}<li>{s}</li>{/each}
            </ul>
          {/each}
        {:else if page === "settings"}
          <h1>{l("Oyun ayarları", "Play settings")}</h1>
          <p class="muted">{l("Yalnız bu oyuna ait; oyunun yanında saklanır, PDF'e girmez.", "This play's alone; kept beside it, never printed.")}</p>
          {#if playSettings}
            <h3>{l("Konuşma hızı", "Pace")}</h3>
            <p>
              <input
                type="number"
                min="30"
                max="400"
                value={playSettings.wpm ?? playSettings.default_wpm}
                onchange={(e) => setWpm(Number(e.currentTarget.value))}
              />
              {l("dakikada kelime; süreler buna göre", "words a minute; the times follow it")}
              {#if playSettings.wpm !== null}
                <button onclick={() => setWpm(null)}>{l(`Varsayılana dön (${playSettings?.default_wpm})`, `Back to the default (${playSettings?.default_wpm})`)}</button>
              {/if}
            </p>
            {#if playSettings.parent}
              <h3>{l("Taslağı olduğu oyun", "A draft of")}</h3>
              <p>{playSettings.parent}</p>
            {/if}
            <h3>{l("Oyunun sözlüğü", "The play's dictionary")}</h3>
            {#if playSettings.words.length === 0}
              <p class="muted">{l("Henüz kelime yok: yazım menüsünden eklenir.", "No words yet: the spelling menu adds them.")}</p>
            {/if}
            <p class="words">
              {#each playSettings.words as w (w)}
                <button title={l("Sözlükten çıkar", "Take it out")} onclick={() => unlearn(w)}>{w} ×</button>
              {/each}
            </p>
          {/if}
        {:else}
          <h1>{l("Şablon olarak kaydet", "Save as a template")}</h1>
          <p>
            {l(
              "Oyunun kapağı, kişileri, perdeleri ve sahneleri kalır; replikleri kalmaz. Yeni oyun açarken şablonlar arasında görünür.",
              "The play's cover, cast, acts and scenes stay; its lines don't. It shows among the templates for a new play.",
            )}
          </p>
          <input bind:value={templateName} placeholder={l("Şablon adı", "Template name")} />
          <button class="primary" disabled={!templateName.trim()} onclick={saveTemplate}>{l("Kaydet", "Save")}</button>
        {/if}
      </section>
    </div>
  {/if}

  {#if characterForm}
    <CharacterFormView
      {tr}
      text={characterForm.text}
      old={characterForm.old}
      start={characterForm.start}
      done={characterDone}
      cancel={() => ((characterForm = null), view.focus())}
    />
  {/if}

  {#if summaryForm}
    <SummaryForm
      {tr}
      title={summaryForm.title}
      start={summaryForm.text}
      done={summaryDone}
      cancel={() => ((summaryForm = null), view.focus())}
    />
  {/if}

  {#if coverForm}
    <CoverForm
      {tr}
      text={view.state.doc.toString()}
      last={coverLast}
      tab={coverForm}
      readOnly={opened.read_only}
      done={writeCover}
      source={() => {
        coverForm = null;
        openCover(view);
      }}
      cancel={() => ((coverForm = null), view.focus())}
    />
  {/if}

  {#if headingForm}
    <HeadingForm
      {tr}
      act={headingForm.act}
      places={headingForm.places}
      times={headingForm.times}
      done={writeHeading}
      cancel={() => ((headingForm = null), view.focus())}
    />
  {/if}

  {#if help}
    <div class="modal" role="dialog" tabindex="-1" onclick={() => (help = "")} onkeydown={() => {}}>
      <!-- svelte-ignore a11y_no_static_element_interactions, a11y_click_events_have_key_events -->
      <div class="sheet" class:wide={help === "keys"} onclick={(e) => e.stopPropagation()}>
        {#if help === "keys"}
          <h2>{l("Tuşlar", "Keys")}</h2>
          <Keys {tr} close={() => (help = "")} />
        {:else}
          <h2>Dactile</h2>
          <p>{l("Tiyatro oyunu yazmak için masaüstü uygulaması. Terminal sürümü: Dactile Light.", "A desktop app for writing stage plays. The terminal one: Dactile Light.")}</p>
          <p class="muted">{version} · GPL-3.0-or-later</p>
        {/if}
      </div>
    </div>
  {/if}
</div>

<style>
  .writer {
    display: flex;
    flex-direction: column;
    height: 100%;
  }
  .body {
    flex: 1;
    min-height: 0;
    display: grid;
    grid-template-columns: 1fr;
    /* One row the height of the window: each side scrolls on its own. */
    grid-template-rows: minmax(0, 1fr);
  }
  .body > :global(aside) {
    min-height: 0;
    overflow: hidden;
  }
  .body {
    position: relative;
  }
  .body.split {
    grid-template-columns: minmax(0, 1fr) var(--pdf-col, minmax(320px, 40%));
  }
  .pdf-edge {
    position: absolute;
    top: 0;
    bottom: 0;
    z-index: 6;
    width: 7px;
    margin-right: -3px;
    cursor: col-resize;
    touch-action: none;
  }
  .pdf-edge:hover {
    background: var(--chrome-line);
  }
  :global(.cm-panels.cm-panels-top) {
    position: relative;
    border: 0 !important;
    background: transparent !important;
  }
  .files {
    display: flex;
    flex-direction: column;
    min-width: 0;
    background: var(--chrome);
    border-left: 1px solid var(--chrome-line);
  }
  .files .head {
    display: flex;
    gap: 6px;
    align-items: center;
    padding: 6px 10px;
    border-bottom: 1px solid var(--chrome-line);
  }
  .files .head button {
    border: 1px solid var(--chrome-line);
    background: var(--chrome);
    border-radius: 4px;
    width: 28px;
    height: 24px;
    cursor: pointer;
  }
  .files .grow {
    flex: 1;
  }
  .files .list {
    flex: 1;
    min-height: 0;
    overflow: auto;
    padding: 8px 10px;
  }
  .left {
    position: relative;
    min-width: 0;
    min-height: 0;
  }
  .desk {
    position: absolute;
    inset: 0;
    overflow: auto;
    background: var(--desk);
    padding: 28px 0 80px;
  }
  .desk.gone {
    visibility: hidden;
  }
  .desk.plain {
    background: var(--page);
    padding-top: 0;
  }
  .desk.plain .page {
    box-shadow: none;
    width: auto;
    max-width: calc((var(--page-width) + 8) * 1em * var(--z));
    min-height: 0;
  }
  .left > :global(.board),
  .left > :global(.casting),
  .left > :global(.memo) {
    position: absolute;
    inset: 0;
  }
  .page {
    --z: 1;
    width: calc(var(--page-width) * 1em * var(--z));
    max-width: calc(100% - 32px);
    margin: 0 auto;
    background: var(--page);
    color: var(--fg);
    box-shadow:
      0 1px 2px rgba(0, 0, 0, 0.12),
      0 6px 24px rgba(0, 0, 0, 0.14);
    padding: calc(3.2em * var(--z)) calc(3.6em * var(--z));
    font-size: calc(var(--page-size) * var(--z));
    min-height: calc(62em * var(--z));
    user-select: text;
  }
  .status {
    display: flex;
    gap: 14px;
    align-items: center;
    height: 26px;
    padding: 0 12px;
    font-size: 12px;
    background: var(--chrome);
    border-top: 1px solid var(--chrome-line);
  }
  .status .grow {
    flex: 1;
  }
  .status .muted {
    color: var(--chrome-muted);
  }
  .status .warn {
    color: var(--accent-ink);
    font-weight: 600;
  }
  .zoom {
    border: 0;
    background: none;
    cursor: pointer;
    color: var(--chrome-muted);
  }

  /* ---- the page's text ---- */
  .page :global(.cm-editor) {
    background: transparent;
    font-family: var(--page-font);
    font-size: 1em;
    line-height: var(--line-spacing);
  }
  .page :global(.cm-scroller) {
    font-family: var(--page-font);
    line-height: var(--line-spacing);
  }
  .page :global(.cm-editor.cm-focused) {
    outline: none;
  }
  .page :global(.cm-content) {
    padding: 0;
  }
  /* `drawSelection` blinks the layer around it. */
  .page :global(.cm-cursor) {
    border-left: 2px solid var(--accent-ink);
  }
  .page :global(.cm-selectionBackground),
  .page :global(.cm-focused .cm-selectionBackground) {
    background: var(--selection) !important;
  }
  .page :global(.cm-line) {
    padding: 0;
  }
  .page :global(.cm-b-line) {
    padding-left: 8.5em;
    text-indent: -8.5em;
    margin-bottom: 0.15em;
  }
  .page :global(.cm-c-name) {
    display: inline-block;
    width: 8.5em;
    text-indent: 0;
    font-family: var(--sans);
    font-size: 0.82em;
    font-weight: 600;
    letter-spacing: 0.04em;
    color: var(--name-ref);
    vertical-align: baseline;
  }
  .page :global(.cm-c-star) {
    color: var(--star);
    font-weight: 800;
  }
  .page :global(.cm-c-ref) {
    font-variant: small-caps;
    color: var(--name-ref);
  }
  .page :global(.cm-b-direction),
  .page :global(.cm-m-dir),
  .page :global(.cm-c-dir) {
    font-style: italic;
    color: var(--muted);
  }
  .page :global(.cm-b-stage) {
    text-align: center;
    font-style: italic;
    padding: 0 3em;
  }
  .page :global(.cm-b-middir) {
    text-align: right;
    font-style: italic;
    color: var(--muted);
  }
  .page :global(.cm-b-act),
  .page :global(.cm-b-scene),
  .page :global(.cm-b-section),
  .page :global(.cm-b-page) {
    text-align: center;
  }
  .page :global(.cm-c-act) {
    display: inline-block;
    white-space: pre-line;
    margin: 1.6em 0 0.8em;
    font-family: var(--sans);
    font-weight: 700;
    letter-spacing: 0.16em;
    color: var(--heading);
    font-size: 1.15em;
  }
  .page :global(.cm-c-scene) {
    display: inline-block;
    white-space: pre-line;
    margin: 1.1em 0 0.5em;
    font-family: var(--sans);
    font-weight: 600;
    color: var(--heading);
    font-size: 0.95em;
    line-height: 1.6;
  }
  .page :global(.cm-c-section) {
    display: inline-block;
    margin: 1.2em 0 0.4em;
    font-family: var(--sans);
    font-weight: 700;
    letter-spacing: 0.15em;
    color: var(--heading);
  }
  .page :global(.cm-c-faint),
  .page :global(.cm-b-comment),
  .page :global(.cm-m-raw) {
    color: var(--faint);
  }
  .page :global(.cm-b-comment) {
    font-family: var(--mono);
    font-size: 0.8em;
  }
  .page :global(.cm-m-under) {
    text-decoration: underline;
  }
  .page :global(.cm-m-em) {
    font-style: italic;
  }
  .page :global(.cm-m-strong) {
    font-weight: 700;
  }
  .page :global(.cm-c-verse) {
    color: var(--faint);
    font-size: 0.8em;
  }
  .page :global(.cm-b-unknown) {
    color: var(--warn);
  }
  .page :global(.cm-b-raw),
  .page :global(.cm-raw),
  .page.source :global(.cm-line) {
    font-family: var(--mono);
    font-size: 0.82em;
    text-indent: 0;
    padding-left: 0;
    text-align: left;
    font-style: normal;
    color: var(--fg);
  }
  .page :global(.cm-raw) {
    background: color-mix(in srgb, var(--selection) 35%, transparent);
    border-radius: 3px;
  }
  .page :global(.cm-note) {
    text-decoration: underline wavy color-mix(in srgb, var(--claret) 70%, transparent);
    text-underline-offset: 3px;
    cursor: pointer;
  }
  .page :global(.cm-note-done) {
    text-decoration-style: dotted;
    text-decoration-color: var(--faint);
  }
  .page :global(.cm-note-picked) {
    background: color-mix(in srgb, var(--claret) 18%, transparent);
  }
  .page :global(.cm-folded) {
    margin: 0.4em 0;
    padding: 0.2em 0.6em;
    border-left: 2px solid var(--accent-ink);
    color: var(--faint);
    font-style: italic;
    cursor: pointer;
  }
  .page :global(.cm-folded:hover) {
    color: var(--accent-ink);
  }
  .page :global(.cm-dimmed .cm-line:not(.cm-here)),
  .page :global(.cm-dimmed .cm-cover) {
    opacity: 0.32;
    transition: opacity 0.15s;
  }
  .page[data-scene-align="left"] :global(.cm-b-scene) {
    text-align: left;
  }
  .page[data-scene-align="right"] :global(.cm-b-scene) {
    text-align: right;
  }
  .page :global(.cm-summary) {
    margin: -0.4em auto 0.8em;
    max-width: 80%;
    text-align: center;
    font-style: italic;
    font-size: 0.9em;
    color: var(--faint);
  }
  .page :global(.cm-misspelt) {
    text-decoration: underline wavy #d23a2f;
    text-decoration-skip-ink: none;
    text-underline-offset: 3px;
  }
  :global(.dactile-menu.cm-tooltip) {
    background: var(--chrome);
    color: var(--chrome-fg);
    border: 1px solid var(--chrome-line);
    border-radius: 6px;
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.2);
    overflow: hidden;
    font-family: var(--sans, system-ui);
  }
  :global(.dactile-menu.cm-tooltip-autocomplete > ul) {
    max-height: 18em;
    min-width: 320px;
    font-family: inherit;
  }
  :global(.dactile-menu.cm-tooltip-autocomplete > ul > li) {
    padding: 3px 10px;
    line-height: 1.5;
  }
  :global(.dactile-menu.cm-tooltip-autocomplete > ul > li[aria-selected]) {
    background: var(--claret);
    color: #f6efe4;
  }
  :global(.dactile-menu .cm-completionLabel) {
    font-family: var(--mono);
    font-size: 13px;
    font-weight: 600;
  }
  :global(.dactile-menu .cm-completionDetail) {
    margin-left: 1em;
    font-family: system-ui, sans-serif;
    font-style: normal;
    font-size: 12.5px;
    opacity: 0.75;
  }
  :global(.dactile-menu completion-section) {
    display: block;
    padding: 4px 10px 2px;
    font-size: 11px;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--chrome-muted);
    border-bottom: 0;
  }
  :global(.spell-menu) {
    position: fixed;
    z-index: 50;
    min-width: 180px;
    padding: 4px;
    display: flex;
    flex-direction: column;
    background: var(--chrome);
    color: var(--chrome-fg);
    border: 1px solid var(--chrome-line);
    border-radius: 6px;
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.2);
    font-size: 13px;
  }
  :global(.spell-menu button) {
    text-align: left;
    background: none;
    border: 0;
    padding: 5px 10px;
    border-radius: 4px;
    cursor: pointer;
    color: inherit;
  }
  :global(.spell-menu button:hover),
  :global(.spell-menu button:focus) {
    background: var(--chrome-2);
    outline: none;
  }
  :global(.spell-menu .spell-sugg) {
    font-family: var(--serif);
    font-size: 14px;
    font-weight: 600;
  }
  :global(.spell-menu .spell-learn) {
    color: var(--accent-ink);
  }
  :global(.spell-menu hr) {
    border: 0;
    border-top: 1px solid var(--chrome-line);
    margin: 3px 0;
  }
  :global(.spell-quiet) {
    padding: 5px 10px;
    color: var(--chrome-muted);
  }
  .page :global(.cm-chip) {
    cursor: pointer;
  }
  .page :global(.cm-cover) {
    text-align: center;
    padding: 1.5em 0 2em;
    margin-bottom: 1.5em;
    border-bottom: 1px solid var(--faint);
    cursor: pointer;
  }
  .page :global(.cm-cover-title) {
    font-size: 2em;
    font-weight: 600;
    color: var(--heading);
  }
  .page :global(.cm-cover-author) {
    margin: 0.3em 0 1em;
    color: var(--muted);
  }
  .page :global(.cm-cover-cast) {
    list-style: none;
    padding: 0 2em;
    text-align: left;
    margin: 0;
    font-size: 0.9em;
    line-height: 1.6;
  }
  .page :global(.cm-cover-group) {
    display: flex;
    justify-content: flex-start;
    align-items: stretch;
    gap: 0.4em;
    margin: 0.3em 0;
  }
  .page :global(.cm-cover-members) {
    text-align: left;
  }
  .page :global(.cm-cover-brace-box) {
    position: relative;
    width: 0.6em;
    flex: none;
  }
  .page :global(.cm-cover-brace) {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    fill: none;
    stroke: currentColor;
    stroke-width: 1.2;
  }
  .page :global(.cm-cover-group-name) {
    align-self: center;
    padding-left: 0.4em;
  }
  .page :global(.cm-cover-hint) {
    margin-top: 1em;
    font: 11px var(--sans);
    color: var(--faint);
  }
  .page :global(.cm-panels) {
    background: var(--chrome);
    color: var(--chrome-fg);
    font-family: var(--sans);
  }

  .words {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
  }
  /* ---- File: the backstage ---- */
  .backstage {
    position: fixed;
    inset: 0;
    z-index: 20;
    display: grid;
    grid-template-columns: 260px 1fr;
    background: var(--chrome);
  }
  .backstage nav {
    display: flex;
    flex-direction: column;
    background: var(--claret);
    padding: 16px 0;
  }
  .backstage nav button {
    display: flex;
    justify-content: space-between;
    text-align: left;
    background: none;
    border: 0;
    color: #f6efe4;
    padding: 10px 22px;
    cursor: pointer;
    font-size: 15px;
  }
  .backstage nav button:hover:not(:disabled),
  .backstage nav button.on {
    background: var(--claret-deep);
  }
  .backstage nav button:disabled {
    opacity: 0.45;
    cursor: default;
  }
  .backstage nav small {
    opacity: 0.7;
  }
  .backstage nav .back {
    font-size: 18px;
    margin-bottom: 14px;
  }
  .backstage section {
    padding: 40px 56px;
    overflow: auto;
    user-select: text;
  }
  .backstage h1 {
    font-family: var(--serif);
    font-weight: 500;
    font-size: 30px;
    margin: 0 0 4px;
  }
  .backstage dl {
    display: grid;
    grid-template-columns: 110px 1fr;
    gap: 6px 12px;
  }
  .backstage dt {
    color: var(--chrome-muted);
  }
  .backstage dd {
    margin: 0;
  }
  .backstage h3 {
    margin: 18px 0 4px;
    color: var(--accent-ink);
    font-size: 14px;
  }
  .backstage ul {
    margin: 0;
    padding-left: 18px;
  }
  .backstage input {
    width: 320px;
    padding: 6px 10px;
    border: 1px solid var(--chrome-line);
    border-radius: 4px;
    margin-right: 8px;
  }
  .primary {
    background: var(--claret);
    color: #f6efe4;
    border: 0;
    border-radius: 4px;
    padding: 7px 16px;
    cursor: pointer;
  }
  .primary:disabled {
    opacity: 0.5;
  }
  .muted {
    color: var(--chrome-muted);
  }

  .modal {
    position: fixed;
    inset: 0;
    z-index: 30;
    background: rgba(0, 0, 0, 0.35);
    display: grid;
    place-items: center;
  }
  .sheet {
    background: var(--chrome);
    border-radius: 8px;
    padding: 20px 28px;
    min-width: 420px;
    box-shadow: 0 12px 40px rgba(0, 0, 0, 0.3);
  }
  .sheet.wide {
    max-width: min(760px, 94vw);
  }
  .sheet h2 {
    margin-top: 0;
    color: var(--accent-ink);
  }
</style>
