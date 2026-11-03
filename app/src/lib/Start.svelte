<script lang="ts">
  // The start screen, as a word processor's "New" page: a column on the
  // left, tiles for a new play along the top, the plays below with a
  // preview of the one picked.
  import { untrack } from "svelte";
  import { api, type Home, type PlayCard, type Preview, type TemplateCard } from "./api";

  let { home, tr, open, reload }: { home: Home; tr: boolean; open: (path: string) => void; reload: () => void } =
    $props();

  type Section = "home" | "new" | "open" | "settings" | "keys";
  let section = $state<Section>("home");
  let chosen = $state<PlayCard | null>(null);
  let preview = $state<Preview | null>(null);
  let making = $state<TemplateCard | null>(null);
  let title = $state("");
  let error = $state("");
  let filter = $state("");

  // The form starts from the settings as they were.
  let author = $state(untrack(() => home.settings.author));
  let playsDir = $state(untrack(() => home.settings.plays_dir));
  let lang = $state("auto");

  const l = (t: string, e: string) => (tr ? t : e);
  const ago = (s: number | null) => {
    if (!s) return "";
    const d = new Date(s * 1000);
    return d.toLocaleDateString(tr ? "tr-TR" : "en-GB", { day: "numeric", month: "short", year: "numeric" });
  };

  const plays = $derived(
    home.plays.filter((p) =>
      (p.title + " " + p.author + " " + p.path).toLocaleLowerCase("tr").includes(filter.toLocaleLowerCase("tr")),
    ),
  );
  const tiles = $derived(home.templates.filter((t) => t.kind !== "own").slice(0, 3));

  async function choose(p: PlayCard) {
    chosen = p;
    preview = null;
    preview = await api.preview(p.path).catch(() => null);
  }

  function startNew(t: TemplateCard) {
    making = t;
    title = "";
    error = "";
  }

  async function create() {
    if (!making) return;
    try {
      open(await api.create(making.index, title));
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
    <div class="brand">
      <img src="/icon.png" alt="" />
      <span>Dactile</span>
    </div>
    {#each SIDE as [id, icon, t, e]}
      <button class:on={section === id} onclick={() => (section = id)}>
        <span class="icon">{icon}</span>{tr ? t : e}
      </button>
    {/each}
    <div class="grow"></div>
    <div class="ver">0.2.0-beta.6</div>
  </nav>

  <main>
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
            <strong>{l("Aç / içe aktar", "Open / import")}</strong>
            <small>{l("bir .tex dosyası", "a .tex file")}</small>
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
        </form>
      {/if}
    {/if}

    {#if section === "home" || section === "open"}
      <div class="plays">
        <div class="list">
          <div class="listhead">
            <h2>{section === "open" ? l("Bütün oyunlar", "All plays") : l("Oyunlar", "Plays")}</h2>
            <input bind:value={filter} placeholder={l("Ara…", "Search…")} />
            {#if section === "open"}
              <button class="plain" onclick={browse}>{l("Başka bir dosya…", "Another file…")}</button>
            {/if}
          </div>
          {#if !plays.length}
            <p class="muted">
              {l(`${home.settings.plays_dir} içinde oyun yok. Yukarıdan yeni bir tane başlat.`, `No plays in ${home.settings.plays_dir}. Start one above.`)}
            </p>
          {/if}
          {#each plays as p}
            <button
              class="row"
              class:on={chosen?.path === p.path}
              onclick={() => choose(p)}
              ondblclick={() => open(p.path)}
            >
              <span class="t">{p.title}{#if p.draft}<em> · {p.draft}</em>{/if}</span>
              <span class="m">{p.acts} {l("perde", "acts")} · {p.scenes} {l("sahne", "scenes")} · ~{p.minutes} {l("dk", "min")}</span>
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
    {/if}

    {#if section === "keys"}
      <h1>{l("Tuşlar", "Keys")}</h1>
      <p>{l("Yazarken F1 bütün listeyi gösterir. Birkaçı:", "While writing, F1 shows them all. A few:")}</p>
      <ul class="keys">
        <li><kbd>Alt</kbd> {l("şeridin harfleri; iki harf her şeyi yapar", "the ribbon's letters; two letters do anything")}</li>
        <li><kbd>F5</kbd> {l("PDF derle ve yanda göster", "build the PDF and show it beside")}</li>
        <li><kbd>Ctrl+G</kbd> {l("pano: sahneleri sürükle bırak", "the board: drag scenes around")}</li>
        <li><kbd>Ctrl +/−</kbd> {l("sayfayı büyüt, küçült", "zoom the page")}</li>
      </ul>
    {/if}
  </main>
</div>

<style>
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
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 0 20px 22px;
    font: 600 22px var(--serif);
  }
  .brand img {
    width: 30px;
    height: 30px;
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
    outline: 2px solid var(--claret);
  }
  .plus {
    font-size: 44px;
    color: var(--claret);
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
    color: var(--claret);
    text-align: center;
  }
  .make {
    display: flex;
    align-items: center;
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
    color: var(--claret);
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
  .row {
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
  .keys {
    line-height: 2;
  }
  kbd {
    font: 12px var(--mono);
    background: var(--chrome-2);
    border: 1px solid var(--chrome-line);
    border-radius: 3px;
    padding: 1px 6px;
  }
</style>
