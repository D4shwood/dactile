<script lang="ts">
  import { onDestroy, onMount, untrack } from "svelte";
  import { EditorState } from "@codemirror/state";
  import { EditorView } from "@codemirror/view";
  import { redo, selectAll, undo } from "@codemirror/commands";
  import { openSearchPanel } from "@codemirror/search";
  import { api, type BuildResult, type Card, type NoteCard, type Opened, type Preview, type Settings } from "./api";
  import { currentView, extensions, insertSnippet, typeBackslash, openCover, pickNote, setNotes, setSource, setView } from "./editor";
  import { BACKSTAGE, type Act } from "./ribbon";
  import { closeMenu, nextWord, showMenu, spelling } from "./spelling";
  import { completion, openMenu } from "./completion";
  import HeadingForm from "./HeadingForm.svelte";
  import Ribbon from "./Ribbon.svelte";
  import Pdf from "./Pdf.svelte";
  import Board from "./Board.svelte";
  import Notes from "./Notes.svelte";

  let {
    opened,
    settings,
    tr,
    close,
  }: { opened: Opened; settings: Settings; tr: boolean; close: () => void } = $props();

  const THEMES = ["paper", "sepia", "night", "contrast", "dark"];

  let host: HTMLDivElement;
  let view: EditorView;
  let ribbon: Ribbon | undefined = $state();
  let dirty = $state(false);
  // Taken once: from here on the writer keeps them, and saves them.
  const first = untrack(() => settings);
  let folded = $state(first.ribbon_folded);
  let zoom = $state(first.page_zoom || 100);
  let theme = $state(first.theme || "paper");
  let source = $state(false);
  let typewriter = $state(false);
  /// What stands beside the page.
  let side = $state<"" | "pdf" | "notes">("");
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
        if (view.state.doc === doc) view.dispatch({ effects: setView.of(v) });
        if (board && view.state.doc === doc) cards = await api.board(doc.toString());
        if (side === "notes" && view.state.doc === doc) await refreshNotes();
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

  function goto(line: number) {
    const n = Math.min(Math.max(line, 1), view.state.doc.lines);
    const pos = view.state.doc.line(n).from;
    board = false;
    view.dispatch({ selection: { anchor: pos }, scrollIntoView: true });
    view.focus();
  }

  function at(pos: number) {
    board = false;
    view.dispatch({ selection: { anchor: pos }, effects: EditorView.scrollIntoView(pos, { y: "start", yMargin: 60 }) });
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

  function setTheme(t: string) {
    theme = t;
    document.documentElement.dataset.theme = t;
    api.set("theme", t);
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
        save().then((ok) => ok && say(l("Kaydedildi.", "Saved.")));
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
      case "pdf-panel":
        side = side === "pdf" ? "" : "pdf";
        // The notes' underlines go with their panel.
        showNotes();
        keepCursor();
        break;
      case "notes":
        openNotes();
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
        openBoard();
        break;
      case "cover":
        board = false;
        openCover(view);
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
      case "close":
        leave();
        break;
    }
    if (!["commands", "find", "spelling", "search", "note-here", "notes-paste", "notes-file"].includes(id)) view.focus();
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

  async function leave() {
    if (dirty && !(await save())) {
      if (!confirm(l("Kaydedilemedi. Yine de kapatayım mı?", "Couldn't save. Close anyway?"))) return;
    }
    await api.close();
    close();
  }

  function onkey(e: KeyboardEvent) {
    const mod = e.ctrlKey || e.metaKey;
    const keys: Record<string, string> = {
      F1: mod ? "fold" : "keys",
      F2: "source",
      F3: "typewriter",
      F5: "build",
      F6: "theme",
      F7: "page",
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
    }
    if (e.altKey && !mod && (e.key === "n" || e.key === "N")) id = "note-here";
    if (e.altKey && !mod && (e.key === "s" || e.key === "S")) id = "spelling";
    if (e.key === "Escape") {
      if (help) return void (help = "");
      if (backstage) return void (backstage = false);
      if (board) return void (board = false);
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
    if (kind === "character") return say(l("Yeni karakter formu yakında.", "The new character form is coming."));
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
    document.documentElement.dataset.theme = theme;
    view = new EditorView({
      parent: host,
      state: EditorState.create({
        doc: opened.text,
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
          spelling({
            tr: () => tr,
            on: () => true,
            missing: (why) => say(why, true),
          }),
          EditorState.readOnly.of(opened.read_only),
          EditorView.updateListener.of((u) => {
            if (u.selectionSet) {
              locate();
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
    // For trying the app out from a script (DACTILE_DEV_EVAL).
    if (import.meta.env.DEV) (window as any).__dactile = { view, key, run, move, openBoard, build, setZoom };
    // To the first line of dialogue, past the cover.
    api.view(opened.text).then((v) => {
      const first = v.blocks.find((b) => b.cls === "line" || b.cls === "scene");
      if (first) view.dispatch({ selection: { anchor: first.to }, scrollIntoView: true });
      view.focus();
    });
    if (opened.read_only) say(l(`Salt okunur: ${opened.holder ?? "başka bir Dactile"} açık tutuyor.`, `Read only: held by ${opened.holder ?? "another Dactile"}.`), true);
  });

  onDestroy(() => {
    closeMenu();
    clearTimeout(viewTimer);
    clearTimeout(saveTimer);
    view?.destroy();
  });

</script>

<svelte:window onkeydown={onkey} />

<div class="writer">
  <Ribbon
    bind:this={ribbon}
    {tr}
    {folded}
    {dirty}
    title={`${opened.name}${opened.read_only ? l(" (salt okunur)", " (read only)") : ""} — Dactile`}
    {run}
    file={() => openBackstage("info")}
  />

  <main class="body" class:split={side !== ""}>
    <div class="left">
      <div class="desk" class:gone={board} class:plain={!sheet} {onwheel}>
        <div class="page" class:source class:typewriter style={`--z: ${zoom / 100}`}>
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
        <Board {tr} {cards} {move} open={at} close={() => (board = false)} />
      {/if}
    </div>
    {#if side === "pdf"}
      <Pdf {tr} result={built} {building} close={() => (side = "")} {goto} />
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
    <span class="muted">{dirty ? l("kaydedilmedi", "unsaved") : l("kaydedildi", "saved")}</span>
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
              (e.act.do === "key" && e.act.id === "save-template" && page === "template")}
            disabled={e.act.do === "soon"}
            onclick={() => {
              if (e.act.do !== "key") return;
              if (e.act.id === "info") page = "info";
              else if (e.act.id === "save-template") page = "template";
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
      <div class="sheet">
        {#if help === "keys"}
          <h2>{l("Tuşlar", "Keys")}</h2>
          <table>
            <tbody>
              <tr><td>Alt</td><td>{l("şerit harfleri: Alt E A bir eylem ekler", "ribbon letters: Alt N A adds an action")}</td></tr>
              <tr><td>Ctrl+S</td><td>{l("kaydet (kendiliğinden de kaydeder)", "save (it saves on its own too)")}</td></tr>
              <tr><td>F5</td><td>{l("PDF derle", "build the PDF")}</td></tr>
              <tr><td>F12</td><td>{l("PDF paneli", "the PDF panel")}</td></tr>
              <tr><td>Ctrl+G</td><td>{l("pano", "the board")}</td></tr>
              <tr><td>Ctrl + / − / 0, Ctrl+{l("tekerlek", "wheel")}</td><td>{l("sayfayı büyüt / küçült", "zoom the page")}</td></tr>
              <tr><td>Enter</td><td>{l("replik sonunda: karşı tarafın repliği", "at a line's end: the other speaker's line")}</td></tr>
              <tr><td>Tab</td><td>{l("konuşmacıdan hemen sonra: sıradaki kişi", "right after the speaker: the next one")}</td></tr>
              <tr><td>\</td><td>{l("bloğu kaynak olarak aç, makro yaz", "open the block as source, write a macro")}</td></tr>
              <tr><td>Ctrl+K</td><td>{l("karakterler, komutlar, makrolar menüsü", "the menu of characters, commands, macros")}</td></tr>
              <tr><td>Alt+S</td><td>{l("sıradaki yazım hatası ve önerileri", "the next misspelt word and its suggestions")}</td></tr>
              <tr><td>Ctrl+N</td><td>{l("notlar paneli", "the notes panel")}</td></tr>
              <tr><td>Alt+N</td><td>{l("seçili yere ya da kelimeye not", "a note on the selection or the word")}</td></tr>
              <tr><td>F7</td><td>{l("kâğıt sayfa ya da pencere boyu metin", "a paper sheet, or the text across the window")}</td></tr>
              <tr><td>F2</td><td>{l("bütün kaynak", "all of the source")}</td></tr>
              <tr><td>F6</td><td>{l("tema", "theme")}</td></tr>
              <tr><td>Ctrl+F1</td><td>{l("şeridi katla", "fold the ribbon")}</td></tr>
            </tbody>
          </table>
        {:else}
          <h2>Dactile</h2>
          <p>{l("Masaüstü uygulaması, ilk kilometre taşı. Terminal sürümü: Dactile Light.", "The desktop app, first milestone. The terminal one: Dactile Light.")}</p>
          <p class="muted">0.2.0-beta.6 · GPL-3.0-or-later</p>
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
  .body.split {
    grid-template-columns: minmax(0, 1fr) minmax(320px, 40%);
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
    max-width: calc(52em * var(--z));
    min-height: 0;
  }
  .left > :global(.board) {
    position: absolute;
    inset: 0;
  }
  .page {
    --z: 1;
    width: calc(44em * var(--z));
    max-width: calc(100% - 32px);
    margin: 0 auto;
    background: var(--page);
    color: var(--fg);
    box-shadow:
      0 1px 2px rgba(0, 0, 0, 0.12),
      0 6px 24px rgba(0, 0, 0, 0.14);
    padding: calc(3.2em * var(--z)) calc(3.6em * var(--z));
    font-size: calc(17px * var(--z));
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
    color: var(--claret);
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
    font-family: var(--serif);
    font-size: 1em;
    line-height: 1.55;
  }
  .page :global(.cm-scroller) {
    font-family: var(--serif);
    line-height: 1.55;
  }
  .page :global(.cm-editor.cm-focused) {
    outline: none;
  }
  .page :global(.cm-content) {
    caret-color: var(--claret);
    padding: 0;
  }
  .page :global(.cm-cursor) {
    border-left: 2px solid var(--claret);
    animation: blink 1.1s steps(1) infinite;
  }
  @keyframes blink {
    50% {
      opacity: 0;
    }
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
    color: var(--claret);
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
    padding: 0;
    margin: 0;
    font-size: 0.9em;
    line-height: 1.6;
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
    color: var(--claret);
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
  .sheet h2 {
    margin-top: 0;
    color: var(--claret);
  }
  .sheet td:first-child {
    font-family: var(--mono);
    font-size: 12px;
    padding-right: 18px;
    white-space: nowrap;
  }
</style>
