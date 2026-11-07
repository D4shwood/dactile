<script lang="ts">
  import Keys from "./Keys.svelte";
  // The start screen, as a word processor's "New" page: a column on the
  // left, tiles for a new play along the top, the plays below with a
  // preview of the one picked.
  import { untrack } from "svelte";
  import { api, type Files, type Home, type Look, type PlayCard, type Preview, type Shown, type TemplateCard } from "./api";
  import FileList from "./FileList.svelte";
  import Import from "./Import.svelte";
  import LookForm from "./LookForm.svelte";
  import { applyLook, pickLook } from "./look";
  import Temporary from "./Temporary.svelte";
  import { getVersion } from "@tauri-apps/api/app";

  let { home, tr, open, reload }: { home: Home; tr: boolean; open: (path: string) => void; reload: () => Promise<void> } =
    $props();

  type Section = "home" | "new" | "open" | "settings" | "keys";
  let section = $state<Section>("home");
  let version = $state("");
  void getVersion().then((v) => (version = v));
  let chosen = $state<PlayCard | null>(null);
  let preview = $state<Preview | null>(null);
  let files = $state<Files | null>(null);
  let making = $state<TemplateCard | null>(null);
  let title = $state("");
  /// Asked of a new play, beside its title; all may stay empty.
  let details = $state<Record<string, string>>({});
  const DETAILS: [string, string, string][] = [
    ["editedby", "Düzenleyen", "Edited by"],
    ["copyrightnotice", "Telif", "Copyright"],
    ["address", "Adres", "Address"],
    ["email", "E-posta", "Email"],
    ["telephone", "Telefon", "Telephone"],
  ];
  let error = $state("");
  let filter = $state("");

  // The form starts from the settings as they were.
  let author = $state(untrack(() => home.settings.author));
  let playsDir = $state(untrack(() => home.settings.plays_dir));
  let lang = $state("auto");
  let look = $state(untrack(() => pickLook(home.settings)));
  let lookError = $state("");

  function setLook<K extends keyof Look>(key: K, value: Look[K]) {
    look[key] = value;
    applyLook(look);
    lookError = "";
    api.set(key, value).catch((e) => (lookError = String(e)));
  }

  const l = (t: string, e: string) => (tr ? t : e);
  const ago = (s: number | null) => {
    if (!s) return "";
    const d = new Date(s * 1000);
    return d.toLocaleDateString(tr ? "tr-TR" : "en-GB", { day: "numeric", month: "short", year: "numeric" });
  };

  /// The list's tab: the plays written here, or those imported.
  let shelf = $state<"mine" | "imported">("mine");
  const importedCount = $derived(home.plays.filter((p) => p.imported).length);
  const plays = $derived(
    home.plays.filter((p) =>
      p.imported === (shelf === "imported") &&
      (p.title + " " + p.author + " " + p.path).toLocaleLowerCase("tr").includes(filter.toLocaleLowerCase("tr")),
    ),
  );
  const tiles = $derived(home.templates.filter((t) => t.kind !== "own").slice(0, 3));

  /// A play and its drafts to the other tab, and the list after it.
  async function moveShelf(p: PlayCard) {
    try {
      await api.playImported(p.path, !p.imported);
      want = p.path;
      await reload();
    } catch (e) {
      error = String(e);
    }
  }

  async function choose(p: PlayCard) {
    chosen = p;
    preview = null;
    files = null;
    api.files(p.path).then(
      (f) => {
        if (chosen === p) files = f;
      },
      (e) => (error = String(e)),
    );
    preview = await api.preview(p.path).catch(() => null);
  }

  async function reloadFiles() {
    const p = chosen;
    if (!p) return;
    const f = await api.files(p.path).catch(() => null);
    if (f && chosen === p) files = f;
  }

  async function openFolder() {
    error = "";
    await api.launch(home.folder.path).catch((e) => (error = String(e)));
  }

  function startNew(t: TemplateCard) {
    making = t;
    title = "";
    details = {};
    error = "";
  }

  async function create() {
    if (!making) return;
    try {
      const filled = Object.fromEntries(Object.entries(details).filter(([, v]) => v.trim()));
      if (Object.keys(filled).length) await api.set("cover_last", { ...home.settings.cover_last, ...filled });
      open(await api.create(making.index, title, filled));
    } catch (e) {
      error = String(e);
    }
  }

  async function example() {
    try {
      open(await api.example());
    } catch (e) {
      error = String(e);
    }
  }

  // ------------------------------------------------------------ importing

  let importing = $state<Shown | null>(null);
  let reading = $state("");
  /// Pages read of how many, in a book read from its images.
  let ocr = $state<[number, number] | null>(null);

  async function importFile() {
    const f = await api.pickImport();
    if (f) await readImport(f);
  }

  async function readImport(f: string) {
    error = "";
    reading = f.split("/").pop() ?? f;
    const off = await api.onOcr((done, total) => (ocr = [done, total]));
    try {
      importing = await api.importRead(f);
    } catch (e) {
      error = String(e);
    }
    off();
    reading = "";
    ocr = null;
  }

  // For trying it from a script, where no file chooser comes up.
  if (import.meta.env.DEV) (window as unknown as { __dactileImport: typeof readImport }).__dactileImport = readImport;

  // ------------------------------------- drafts, copies, names, the trash

  type PlayForm = { kind: "fork" | "copy" | "rename"; title: string; draft: string; file: string; touched: boolean; error: string };
  let form = $state<PlayForm | null>(null);
  /// To pick once the list has it.
  let want = $state<string | null>(null);
  /// Delete pressed once, with the drafts that stay.
  let arming = $state<number | null>(null);
  let thrown = $state("");

  async function startForm(kind: PlayForm["kind"]) {
    const p = chosen!;
    arming = null;
    const s =
      kind === "fork"
        ? await api.forkStart(p.path, p.draft, new Date().toLocaleDateString("tr-TR", { day: "2-digit", month: "2-digit", year: "numeric" }))
        : kind === "copy"
          ? await api.copyStart(p.title)
          : await api.renameStart(p.path);
    form = { kind, ...s, touched: false, error: "" };
  }

  /// The file name follows the title or the draft until edited by hand.
  async function follow() {
    const f = form!;
    if (f.touched || f.kind === "rename") return;
    const file = f.kind === "fork" ? await api.forkFile(chosen!.path, f.draft) : await api.titleFile(f.title);
    if (form === f) f.file = file;
  }

  async function submitForm() {
    const f = form!;
    const p = chosen!.path;
    try {
      const made =
        f.kind === "rename"
          ? await api.playRename(p, f.title, f.file)
          : f.kind === "fork"
            ? await api.playMake(p, null, f.draft, f.file)
            : await api.playMake(p, f.title, null, f.file);
      form = null;
      // After the list: a new title under the same path is a new card.
      await reload();
      want = made;
    } catch (e) {
      f.error = String(e);
    }
  }

  async function throwAway() {
    const p = chosen!;
    form = null;
    if (arming === null) {
      arming = await api.playDrafts(p.path);
      return;
    }
    arming = null;
    try {
      await api.playThrow(p.path);
      thrown = p.title;
      await reload();
    } catch (e) {
      error = String(e);
    }
  }

  // ------------------------------------- several plays, and the right click

  /// The paths chosen with Shift or Ctrl, past the one on view.
  let picks = $state<string[]>([]);
  let anchor = $state<string | null>(null);
  /// The menu of a right click.
  let menu = $state<{ x: number; y: number } | null>(null);
  /// Asking the name of a book to gather the chosen plays under.
  let gathering = $state<string | null>(null);
  let arming2 = $state(false);
  let working = $state("");

  function clickRow(p: PlayCard, e: MouseEvent) {
    form = null;
    arming = null;
    arming2 = false;
    gathering = null;
    if (e.shiftKey && anchor) {
      const a = plays.findIndex((x) => x.path === anchor);
      const b = plays.indexOf(p);
      picks = plays.slice(Math.min(a, b), Math.max(a, b) + 1).map((x) => x.path);
    } else if (e.ctrlKey || e.metaKey) {
      picks = picks.includes(p.path) ? picks.filter((x) => x !== p.path) : [...picks, p.path];
      anchor = p.path;
    } else {
      picks = [];
      anchor = p.path;
    }
    void choose(p);
  }

  function rightClick(p: PlayCard, e: MouseEvent) {
    e.preventDefault();
    if (!picks.includes(p.path)) {
      picks = [];
      anchor = p.path;
      void choose(p);
    }
    form = null;
    arming = null;
    menu = { x: Math.min(e.clientX, innerWidth - 230), y: Math.min(e.clientY, innerHeight - 300) };
  }

  /// A menu's choice: the menu goes, whatever it does.
  const pick = (run: () => void) => () => {
    menu = null;
    run();
  };

  async function throwPicks() {
    if (!arming2) {
      arming2 = true;
      return;
    }
    arming2 = false;
    try {
      for (const path of picks) await api.playThrow(path);
      picks = [];
      await reload();
    } catch (e) {
      error = String(e);
    }
  }

  async function gather() {
    if (gathering === null) return;
    try {
      const inOrder = plays.filter((p) => picks.includes(p.path)).map((p) => p.path);
      want = await api.bookGather(inOrder, gathering);
      gathering = null;
      picks = [];
      await reload();
    } catch (e) {
      error = String(e);
    }
  }

  async function bookPdf(p: PlayCard) {
    working = l("Kitap derleniyor…", "Building the book…");
    try {
      await api.launch(await api.bookPdf(p.path));
    } catch (e) {
      error = String(e);
    }
    working = "";
  }

  async function restore() {
    try {
      const back = await api.playRestore();
      await reload();
      want = back;
    } catch (e) {
      error = String(e);
    }
    thrown = "";
  }

  async function browse() {
    const f = await api.pick();
    if (f) open(f);
  }

  async function saveSettings() {
    await api.set("author", author);
    await api.set("plays_dir", playsDir);
    await api.set("lang", lang);
    reload();
  }

  $effect(() => {
    const wanted = want && home.plays.find((p) => p.path === want);
    if (wanted) {
      want = null;
      shelf = wanted.imported ? "imported" : "mine";
      choose(wanted);
    }
    if (chosen && !home.plays.some((p) => p.path === chosen!.path)) chosen = null;
    if (chosen && !plays.some((p) => p.path === chosen!.path)) chosen = null;
    if (!chosen && plays.length) choose(plays[0]);
  });

  const SIDE: [Section, string, string, string][] = [
    ["home", "⌂", "Giriş", "Home"],
    ["new", "✚", "Yeni", "New"],
    ["open", "▤", "Aç", "Open"],
    ["settings", "⚙", "Ayarlar", "Settings"],
    ["keys", "⌨", "Tuşlar", "Keys"],
  ];
</script>

<div class="start">
  <nav class="side">
    <!-- The terminal app's masthead: the name spaced out, a typewriter's
         cursor blinking after it, and what it is typing itself out. -->
    <div class="brand" aria-label="Dactile">
      <div class="name">D A C T I L E<span class="caret"></span></div>
      <div class="what">Dynamic Stagewright</div>
    </div>
    {#each SIDE as [id, icon, t, e]}
      <button class:on={section === id} onclick={() => (section = id)}>
        <span class="icon">{icon}</span>{tr ? t : e}
      </button>
    {/each}
    <div class="grow"></div>
    <div class="ver">{version}</div>
  </nav>

  <main>
    {#if home.folder.temporary && section !== "settings" && section !== "keys"}
      <Temporary {tr} shown={home.folder.shown} label={l("Oyunlar klasörünü değiştir", "Change the plays folder")} act={() => (section = "settings")} />
    {/if}
    {#if section === "home" || section === "new"}
      <h1>{section === "home" ? l("İyi yazmalar", "Good writing") : l("Yeni oyun", "New play")}</h1>
      <div class="tiles" class:all={section === "new"}>
        {#each section === "new" ? home.templates : tiles as t}
          <button class="tile" class:picked={making === t} onclick={() => startNew(t)}>
            <div class="sheet" class:blank={t.kind === "blank"}>
              {#if t.kind === "blank"}
                <span class="plus">+</span>
              {:else}
                <span class="lines"><i></i><i></i><i class="short"></i><i></i><i class="short"></i></span>
              {/if}
            </div>
            <strong>{t.name}</strong>
            <small>{t.about}</small>
          </button>
        {/each}
        {#if section === "home"}
          <button class="tile" onclick={browse}>
            <div class="sheet open"><span class="plus">⇲</span></div>
            <strong>{l("Aç", "Open")}</strong>
            <small>{l("bir .tex dosyası", "a .tex file")}</small>
          </button>
          <button class="tile" onclick={importFile}>
            <div class="sheet open"><span class="plus">⇱</span></div>
            <strong>{l("İçe aktar", "Import")}</strong>
            <small>.docx .odt .pdf .md .txt</small>
          </button>
          <button class="tile" onclick={example}>
            <div class="sheet example"><span class="ex">Kar<br />Masalı</span></div>
            <strong>{l("Örnek oyun", "Example play")}</strong>
            <small>{l("bir kopyası, oynamak için", "a copy to play with")}</small>
          </button>
        {/if}
      </div>

      {#if making}
        <form
          class="make"
          onsubmit={(e) => {
            e.preventDefault();
            create();
          }}
        >
          <span>{making.name}:</span>
          <!-- svelte-ignore a11y_autofocus -->
          <input bind:value={title} placeholder={l("Oyunun adı", "The play's title")} autofocus />
          <button class="primary" disabled={!title.trim()}>{l("Oluştur", "Create")}</button>
          <button type="button" class="plain" onclick={() => (making = null)}>{l("Vazgeç", "Cancel")}</button>
          {#if error}<span class="error">{error}</span>{/if}
          <div class="details">
            {#each DETAILS as [k, t, e]}
              <label>{l(t, e)}<input value={details[k] ?? ""} oninput={(ev) => (details[k] = ev.currentTarget.value)} /></label>
            {/each}
            {#if Object.values(home.settings.cover_last ?? {}).some((v) => v.trim())}
              <button type="button" class="plain" onclick={() => (details = { ...home.settings.cover_last, ...Object.fromEntries(Object.entries(details).filter(([, v]) => v.trim())) })}>{l("Son girilenleri kullan", "Use the last ones")}</button>
            {/if}
          </div>
        </form>
      {/if}
    {/if}

    {#if section === "home"}
      <p class="folder">
        <span>{l("Oyunlar klasörü", "Plays folder")}:</span>
        {#if home.folder.exists}
          <button class="link" title={home.folder.path} onclick={openFolder}>{home.folder.shown}</button>
        {:else}
          <span class="where" title={home.folder.path}>{home.folder.shown}</span>
          <span class="muted">{l("henüz yok: ilk oyunla oluşur.", "not there yet: the first play makes it.")}</span>
        {/if}
      </p>
    {/if}

    {#if section === "home" || section === "open"}
      <div class="plays">
        <div class="list">
          <div class="listhead">
            <h2>{section === "open" ? l("Bütün oyunlar", "All plays") : l("Oyunlar", "Plays")}</h2>
            <input bind:value={filter} placeholder={l("Ara…", "Search…")} />
            {#if section === "open"}
              <button class="plain" onclick={browse}>{l("Başka bir dosya…", "Another file…")}</button>
              <button class="plain" onclick={importFile}>{l("İçe aktar…", "Import…")}</button>
            {/if}
          </div>
          {#if reading}
            <p class="muted">
              {l(`${reading} okunuyor…`, `Reading ${reading}…`)}
              {#if ocr}
                {l(`OCR ${ocr[0]}/${ocr[1]} sayfa`, `OCR ${ocr[0]}/${ocr[1]} pages`)}
                <button onclick={() => api.importStop()}>{l("Durdur", "Stop")}</button>
              {/if}
            </p>
          {/if}
          {#if error && !making}
            <p class="error">{error}</p>
          {/if}
          {#if thrown}
            <p class="undo">
              {l(`«${thrown}» çöp kutusunda.`, `«${thrown}» is in the trash.`)}
              <button class="plain" onclick={restore}>{l("Geri al", "Undo")}</button>
            </p>
          {/if}
          <div class="shelves" role="tablist">
            <button role="tab" aria-selected={shelf === "mine"} class:on={shelf === "mine"} onclick={() => (shelf = "mine")}>
              {l("Oyunlarım", "My plays")} <small>{home.plays.length - importedCount}</small>
            </button>
            <button role="tab" aria-selected={shelf === "imported"} class:on={shelf === "imported"} onclick={() => (shelf = "imported")}>
              {l("İçe aktarılanlar", "Imported")} <small>{importedCount}</small>
            </button>
          </div>
          {#if shelf === "imported" && !plays.length}
            <p class="muted">
              {l("İçe aktarılan oyun yok. Eski bir oyunu buraya almak için ona sağ tıkla.", "Nothing imported. To bring an older play here, right click it.")}
            </p>
          {:else if !plays.length}
            <p class="muted">
              {l(`${home.settings.plays_dir} içinde oyun yok. Yukarıdan yeni bir tane başlat.`, `No plays in ${home.settings.plays_dir}. Start one above.`)}
            </p>
          {/if}
          {#if picks.length > 1}
            <div class="several">
              <span>{l(`${picks.length} oyun seçili`, `${picks.length} plays chosen`)}</span>
              {#if gathering !== null}
                <input aria-label={l("Kitabın adı", "The book's title")} bind:value={gathering} />
                <button class="plain" onclick={gather}>{l("Kitap yap", "Make the book")}</button>
                <button class="plain" onclick={() => (gathering = null)}>{l("Vazgeç", "Cancel")}</button>
              {:else}
                <button class="plain" onclick={() => (gathering = "")}>{l("Kitapta topla", "Gather in a book")}</button>
                <button class="plain" onclick={throwPicks}>{arming2 ? l("Evet, çöpe at", "Yes, to the trash") : l("Çöpe at", "To the trash")}</button>
                <button class="plain" onclick={() => ((picks = []), (arming2 = false))}>{l("Seçimi bırak", "Clear")}</button>
              {/if}
            </div>
          {/if}
          {#if working}<p class="muted">{working}</p>{/if}
          {#each plays as p}
            <button
              class="row"
              class:on={chosen?.path === p.path}
              class:picked={picks.includes(p.path)}
              style:padding-left="{10 + p.depth * 22}px"
              onclick={(e) => clickRow(p, e)}
              ondblclick={() => open(p.path)}
              oncontextmenu={(e) => rightClick(p, e)}
            >
              {#if p.lead.length}
                <span class="lead" aria-hidden="true">{#each p.lead as cell}<i class={cell}></i>{/each}</span>
              {/if}
              <span class="t">{p.title}{#if p.draft}<em> · {p.draft}</em>{/if}{#if p.book}<em> · {l("kitap", "book")}</em>{/if}</span>
              <span class="m">{p.acts ? `${p.acts} ${l("perde", "acts")}` : l("Tek Perde", "One act")} · {p.scenes} {l("sahne", "scenes")} · ~{p.minutes} {l("dk", "min")}</span>
              <span class="d">{ago(p.modified)}{p.elsewhere ? " · " + l("dışarıda", "elsewhere") : ""}</span>
            </button>
          {/each}
        </div>
        <aside class="preview">
          {#if chosen}
            <div class="paper">
              <div class="ptitle">{preview?.title ?? chosen.title}</div>
              <div class="pauthor">{preview?.author ?? chosen.author}</div>
              {#if preview}
                <ul class="pcast">
                  {#each preview.cast.slice(0, 8) as c}<li>{c}</li>{/each}
                </ul>
                {#each preview.acts.slice(0, 4) as [act, scenes]}
                  {#if act}<div class="pact">{act}</div>{/if}
                  {#each scenes.slice(0, 5) as s}<div class="pscene">{s}</div>{/each}
                {/each}
              {/if}
            </div>
            <div class="facts">
              {chosen.characters} {l("kişi", "characters")} · {chosen.words} {l("söz", "words")}
            </div>
            <button class="primary wide" onclick={() => open(chosen!.path)}>{l("Aç", "Open")}</button>
            <div class="tools">
              <button class="plain" onclick={() => startForm("fork")} title={l("Bu hâlinden yeni bir taslak; hangi oyundan geldiğini hatırlar", "A new draft from it as it is; it remembers where it came from")}>{l("Yeni taslak", "New draft")}</button>
              <button class="plain" onclick={() => startForm("copy")}>{l("Kopya", "Copy")}</button>
              <button class="plain" onclick={() => startForm("rename")}>{l("Adını değiştir", "Rename")}</button>
              <button class="plain" onclick={throwAway}>{l("Çöpe at", "To the trash")}</button>
            </div>
            {#if arming !== null}
              <p class="undo">
                {l("Çöp kutusuna gitsin mi?", "To the trash?")}
                {#if arming}{l(`Taslakları (${arming}) yerinde kalır.`, `Its drafts (${arming}) stay.`)}{/if}
                <button class="plain" onclick={throwAway}>{l("Evet", "Yes")}</button>
                <button class="plain" onclick={() => (arming = null)}>{l("Vazgeç", "Cancel")}</button>
              </p>
            {/if}
            {#if form}
              <form class="playform" onsubmit={(e) => (e.preventDefault(), void submitForm())}>
                {#if form.kind === "fork"}
                  <label>{l("Taslak no", "Draft")}<input bind:value={form.draft} oninput={follow} /></label>
                {:else}
                  <label>{l("Başlık", "Title")}<input bind:value={form.title} oninput={follow} /></label>
                {/if}
                <label>{l("Dosya", "File")}<input bind:value={form.file} oninput={() => (form!.touched = true)} /></label>
                {#if form.error}<span class="error">{form.error}</span>{/if}
                <div>
                  <button class="primary">{form.kind === "rename" ? l("Değiştir", "Rename") : l("Oluştur", "Make")}</button>
                  <button type="button" class="plain" onclick={() => (form = null)}>{l("Vazgeç", "Cancel")}</button>
                </div>
              </form>
            {/if}
            {#if files}
              {#if files.temporary && !home.folder.temporary}
                <Temporary {tr} shown={files.folder.shown} />
              {/if}
              <h3 class="fileshead">{l("Bu oyunun dosyaları", "This play's files")}</h3>
              <FileList {tr} {files} {open} reload={reloadFiles} />
            {/if}
            <div class="path">{chosen.path}</div>
          {/if}
        </aside>
      </div>
    {/if}

    {#if section === "settings"}
      <h1>{l("Ayarlar", "Settings")}</h1>
      <form
        class="settings"
        onsubmit={(e) => {
          e.preventDefault();
          saveSettings();
        }}
      >
        <label>{l("Yazar", "Author")} <input bind:value={author} /></label>
        <label>{l("Oyunlar klasörü", "Plays folder")} <input bind:value={playsDir} /></label>
        <label
          >{l("Dil", "Language")}
          <select bind:value={lang}>
            <option value="auto">{l("kendiliğinden", "automatic")}</option>
            <option value="tr">Türkçe</option>
            <option value="en">English</option>
          </select>
        </label>
        <button class="primary">{l("Kaydet", "Save")}</button>
      </form>
      <h2 class="lookhead">{l("Görünüş", "Appearance")}</h2>
      <LookForm {tr} {look} set={setLook} />
      {#if lookError}<p class="error">{lookError}</p>{/if}
    {/if}

    {#if section === "keys"}
      <h1>{l("Tuşlar", "Keys")}</h1>
      <Keys {tr} />
    {/if}
  </main>
</div>

{#if importing}
  <Import {tr} start={importing} {open} close={() => (importing = null)} />
{/if}

{#if menu && chosen}
  <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
  <div class="scrim" onclick={() => (menu = null)} oncontextmenu={(e) => (e.preventDefault(), (menu = null))}></div>
  <div class="context" role="menu" style:left="{menu.x}px" style:top="{menu.y}px">
    {#if picks.length > 1}
      <button role="menuitem" onclick={pick(() => (gathering = ""))}>{l("Kitapta topla", "Gather in a book")}</button>
      <button role="menuitem" onclick={pick(throwPicks)}>{l(`${picks.length} oyunu çöpe at`, `${picks.length} plays to the trash`)}</button>
    {:else}
      {@const p = chosen}
      <button role="menuitem" onclick={pick(() => open(p.path))}>{l("Aç", "Open")}</button>
      {#if p.book}
        <button role="menuitem" onclick={pick(() => bookPdf(p))}>{l("Kitabın PDF'ini oluştur", "Make the book's PDF")}</button>
      {/if}
      <button role="menuitem" onclick={pick(() => startForm("fork"))}>{l("Yeni taslak", "New draft")}</button>
      <button role="menuitem" onclick={pick(() => startForm("copy"))}>{l("Kopya", "Copy")}</button>
      <button role="menuitem" onclick={pick(() => startForm("rename"))}>{l("Adını değiştir", "Rename")}</button>
      <button role="menuitem" onclick={pick(() => api.reveal(p.path))}>{l("Dosya gezgininde göster", "Show in the file manager")}</button>
      <button role="menuitem" onclick={pick(() => moveShelf(p))}>
        {p.imported ? l("Oyunlarıma taşı", "Move to my plays") : l("İçe aktarılanlara taşı", "Move to imported")}
      </button>
      <button role="menuitem" onclick={pick(throwAway)}>{l("Çöpe at", "To the trash")}</button>
    {/if}
  </div>
{/if}

<style>
  .scrim {
    position: fixed;
    inset: 0;
    z-index: 40;
  }
  .context {
    position: fixed;
    z-index: 41;
    min-width: 210px;
    padding: 4px;
    display: flex;
    flex-direction: column;
    background: var(--chrome);
    border: 1px solid var(--chrome-line);
    border-radius: 6px;
    box-shadow: 0 6px 20px rgba(0, 0, 0, 0.35);
  }
  .context button {
    all: unset;
    padding: 6px 10px;
    border-radius: 4px;
    cursor: pointer;
    font-size: 13px;
  }
  .context button:hover,
  .context button:focus-visible {
    background: var(--chrome-line);
  }
  .several {
    display: flex;
    gap: 8px;
    align-items: center;
    flex-wrap: wrap;
    padding: 6px 10px;
    margin-bottom: 6px;
    border: 1px solid var(--chrome-line);
    border-radius: 6px;
  }
  .row.picked {
    outline: 1px solid var(--claret);
    outline-offset: -1px;
  }
  .folder {
    display: flex;
    gap: 8px;
    align-items: baseline;
    min-width: 0;
    margin: 24px 0 0;
    font-size: 13px;
    color: var(--chrome-muted);
  }
  .folder > span:first-child {
    flex: none;
  }
  .folder .link,
  .folder .where {
    min-width: 0;
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }
  .folder .link {
    padding: 0;
    border: 0;
    background: none;
    color: var(--accent-ink);
    font: inherit;
    cursor: pointer;
  }
  .folder .link:hover {
    text-decoration: underline;
  }
  .folder .muted {
    flex: none;
  }
  .lookhead {
    margin: 28px 0 10px;
  }
  .fileshead {
    margin: 16px 0 4px;
    font-size: 13px;
  }
  .tools {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
    margin-top: 8px;
  }
  .tools button {
    font-size: 12px;
  }
  .undo {
    font-size: 13px;
    margin: 8px 0;
  }
  .playform {
    display: flex;
    flex-direction: column;
    gap: 6px;
    margin-top: 8px;
    font-size: 12px;
  }
  .playform label {
    display: flex;
    flex-direction: column;
    gap: 2px;
    color: var(--chrome-muted);
  }
  .playform input {
    padding: 3px 6px;
    border: 1px solid var(--chrome-line);
    border-radius: 4px;
    background: var(--chrome);
    color: var(--chrome-fg);
  }
  .start {
    display: grid;
    grid-template-columns: 210px 1fr;
    height: 100%;
  }
  .side {
    display: flex;
    flex-direction: column;
    background: var(--claret);
    color: #f6efe4;
    padding: 18px 0;
  }
  .brand {
    margin: -18px 0 14px;
    padding: 18px 20px 14px;
    background: #151515;
    font-family: var(--mono, ui-monospace, "DejaVu Sans Mono", monospace);
  }
  .brand .name {
    color: #d8cfc0;
    font-weight: 700;
    font-size: 19px;
    white-space: nowrap;
  }
  .caret {
    display: inline-block;
    width: 0.6em;
    height: 1.05em;
    margin-left: 0.35em;
    vertical-align: -0.15em;
    background: #c4505a;
    animation: blink 1.2s steps(1) infinite;
  }
  .brand .what {
    margin-top: 4px;
    color: #8f877b;
    font-size: 12px;
    white-space: nowrap;
    overflow: hidden;
    width: 19ch;
    animation: type 0.55s steps(19) 0.15s both;
  }
  @keyframes blink {
    50% {
      opacity: 0;
    }
  }
  @keyframes type {
    from {
      width: 0;
    }
  }
  /* The cursor blinks all the same. */
  @media (prefers-reduced-motion: reduce) {
    .brand .what {
      animation: none;
    }
  }
  .side button {
    display: flex;
    align-items: center;
    gap: 12px;
    background: none;
    border: 0;
    color: inherit;
    text-align: left;
    padding: 10px 22px;
    cursor: pointer;
    font-size: 15px;
  }
  .side button:hover,
  .side button.on {
    background: var(--claret-deep);
  }
  .side .icon {
    width: 18px;
    text-align: center;
  }
  .grow {
    flex: 1;
  }
  .ver {
    padding: 0 22px;
    font-size: 11px;
    opacity: 0.7;
  }
  main {
    overflow: auto;
    padding: 30px 40px;
  }
  h1 {
    font: 500 28px var(--serif);
    margin: 0 0 18px;
  }
  h2 {
    font-size: 15px;
    margin: 0;
  }
  .tiles {
    display: flex;
    gap: 18px;
    flex-wrap: wrap;
  }
  .tile {
    display: flex;
    flex-direction: column;
    gap: 3px;
    width: 140px;
    background: none;
    border: 0;
    padding: 0;
    text-align: left;
    cursor: pointer;
  }
  .tile small {
    color: var(--chrome-muted);
    font-size: 11.5px;
  }
  .sheet {
    height: 180px;
    border-radius: 3px;
    background: #fbf8f1;
    border: 1px solid var(--chrome-line);
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.1);
    display: grid;
    place-items: center;
    margin-bottom: 6px;
    transition: transform 0.12s;
  }
  .tile:hover .sheet,
  .tile.picked .sheet {
    border-color: var(--claret);
    transform: translateY(-2px);
  }
  .tile.picked .sheet {
    outline: 2px solid var(--accent-ink);
  }
  .plus {
    font-size: 44px;
    color: var(--accent-ink);
    font-weight: 300;
  }
  .lines {
    display: flex;
    flex-direction: column;
    gap: 9px;
    width: 70%;
  }
  .lines i {
    display: block;
    height: 4px;
    background: #d8cfc0;
    border-radius: 2px;
  }
  .lines i:first-child {
    width: 50%;
    margin: 0 auto 8px;
    background: var(--claret);
  }
  .lines i.short {
    width: 60%;
  }
  .ex {
    font: 600 20px/1.1 var(--serif);
    color: var(--accent-ink);
    text-align: center;
  }
  .make {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 10px;
    margin: 18px 0 0;
    padding: 12px 14px;
    background: var(--chrome-2);
    border-radius: 6px;
  }
  .make input,
  .listhead input,
  .settings input,
  .settings select {
    padding: 6px 10px;
    border: 1px solid var(--chrome-line);
    border-radius: 4px;
    background: var(--chrome);
    color: var(--chrome-fg);
  }
  .make input {
    width: 320px;
  }
  .error {
    color: var(--accent-ink);
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
  .primary.wide {
    width: 100%;
    margin-top: 12px;
  }
  .plain {
    background: none;
    border: 1px solid var(--chrome-line);
    border-radius: 4px;
    padding: 6px 12px;
    cursor: pointer;
  }
  .plays {
    display: grid;
    grid-template-columns: minmax(0, 1fr) 300px;
    gap: 28px;
    margin-top: 32px;
  }
  .listhead {
    display: flex;
    align-items: center;
    gap: 12px;
    margin-bottom: 8px;
  }
  .listhead input {
    margin-left: auto;
    width: 200px;
  }
  .shelves {
    display: flex;
    gap: 2px;
    margin-bottom: 6px;
    border-bottom: 1px solid var(--chrome-line);
  }
  .shelves button {
    margin-bottom: -1px;
    padding: 6px 12px;
    color: var(--chrome-muted);
    background: none;
    border: 0;
    border-bottom: 2px solid transparent;
    cursor: pointer;
  }
  .shelves button.on {
    color: var(--chrome-fg);
    font-weight: 600;
    border-bottom-color: var(--accent-ink);
  }
  .shelves small {
    margin-left: 3px;
    opacity: 0.7;
  }
  .lead {
    position: absolute;
    inset: 0 auto -1px 10px;
    display: flex;
    pointer-events: none;
  }
  .lead i {
    position: relative;
    width: 22px;
  }
  .lead .pipe::before,
  .lead .mid::before,
  .lead .top::before {
    content: "";
    position: absolute;
    left: 7px;
    top: 0;
    bottom: 0;
    border-left: 1px solid var(--chrome-muted);
  }
  .lead .top::before {
    top: 19px;
  }
  .lead .mid::after,
  .lead .top::after {
    content: "";
    position: absolute;
    left: 7px;
    top: 19px;
    width: 12px;
    border-top: 1px solid var(--chrome-muted);
  }
  .row {
    position: relative;
    display: grid;
    grid-template-columns: 1fr auto;
    grid-template-rows: auto auto;
    width: 100%;
    background: none;
    border: 0;
    border-bottom: 1px solid var(--chrome-line);
    text-align: left;
    padding: 9px 10px;
    cursor: pointer;
    border-radius: 4px;
  }
  .row:hover {
    background: var(--chrome-2);
  }
  .row.on {
    background: color-mix(in srgb, var(--claret) 12%, transparent);
  }
  .row .t {
    font: 500 16px var(--serif);
  }
  .row .t em {
    font-size: 13px;
    color: var(--chrome-muted);
  }
  .row .m {
    grid-row: 2;
    font-size: 12px;
    color: var(--chrome-muted);
  }
  .row .d {
    grid-row: 1 / 3;
    grid-column: 2;
    align-self: center;
    font-size: 12px;
    color: var(--chrome-muted);
  }
  .preview .paper {
    background: #fbf8f1;
    color: #221f1b;
    aspect-ratio: 1 / 1.414;
    padding: 26px 22px;
    box-shadow: 0 4px 18px rgba(0, 0, 0, 0.15);
    font-family: var(--serif);
    overflow: hidden;
    text-align: center;
  }
  .ptitle {
    font-size: 21px;
    font-weight: 600;
    color: #932c34;
  }
  .pauthor {
    font-size: 13px;
    color: #6e665a;
    margin-bottom: 12px;
  }
  .pcast {
    text-align: left;
    list-style: none;
    padding: 0;
    margin: 0 0 12px;
    font-size: 11px;
    line-height: 1.5;
  }
  .pact {
    font: 700 10px var(--sans);
    letter-spacing: 0.12em;
    color: #932c34;
    margin-top: 8px;
  }
  .pscene {
    font-size: 11px;
  }
  .facts {
    margin-top: 10px;
    font-size: 12px;
    color: var(--chrome-muted);
  }
  .path {
    margin-top: 8px;
    font-size: 11px;
    color: var(--chrome-muted);
    word-break: break-all;
  }
  .muted {
    color: var(--chrome-muted);
  }
  .settings {
    display: grid;
    gap: 14px;
    max-width: 480px;
  }
  .settings label {
    display: grid;
    gap: 4px;
    font-size: 13px;
    color: var(--chrome-muted);
  }
  .settings .primary {
    justify-self: start;
  }
  .make .details {
    flex-basis: 100%;
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(170px, 1fr));
    gap: 8px 10px;
    margin-top: 8px;
  }
  .make .details label {
    display: flex;
    flex-direction: column;
    gap: 3px;
    font-size: 12px;
    color: var(--chrome-muted);
  }
  .make .details input {
    min-width: 0;
    width: 100%;
  }
</style>
