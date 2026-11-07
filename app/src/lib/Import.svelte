<script lang="ts">
  // The preview of an imported document: the names found to put right, the
  // document's parts to mark (a book's cover, its contents, where the play
  // starts), and the play as it was read, before anything is written.
  import { onMount, tick, untrack } from "svelte";
  import { api, type ImportEdit, type PlayLine, type Shown, type Written } from "./api";

  let {
    tr,
    start,
    open,
    close,
  }: {
    tr: boolean;
    start: Shown;
    /// The play is written: on to it.
    open: (path: string) => void;
    close: () => void;
  } = $props();

  const l = (t: string, e: string) => (tr ? t : e);
  let s = $state(untrack(() => start));
  let error = $state("");
  /// Remounts the fields: a refused name goes back to what it was.
  let round = $state(0);

  /// Pages read of how many, while the book is read from its images.
  let ocr = $state<[number, number] | null>(null);

  async function readAgain() {
    ocr = [0, 0];
    const off = await api.onOcr((done, total) => (ocr = [done, total]));
    try {
      s = await api.importRead(s.file, true);
      doc = await api.importDoc();
      overview = s.plays.length > 1;
      if (overview) await list();
      error = "";
      round++;
    } catch (err) {
      error = String(err);
    }
    off();
    ocr = null;
  }

  async function pickPlay(e: Event) {
    await edit({ kind: "play", i: Number((e.currentTarget as HTMLSelectElement).value) });
    doc = await api.importDoc();
  }

  /// The volume's list: first on view for a book of several plays.
  let volume = $state<PlayLine[]>([]);
  let overview = $state(untrack(() => start.plays.length > 1));
  /// The plays to write; none is left out until asked.
  let chosen = $state<boolean[]>([]);
  let written = $state<Written[] | null>(null);
  let writing = $state(false);
  const count = $derived(chosen.filter(Boolean).length);

  async function list() {
    const was = chosen;
    const same = was.length === s.plays.length;
    volume = await api.importVolume();
    chosen = volume.map((_, i) => (same ? was[i] : true));
  }

  async function toOverview() {
    await list();
    overview = true;
  }

  async function openPlay(i: number) {
    await edit({ kind: "play", i });
    doc = await api.importDoc();
    overview = false;
  }

  async function dropPlay(i: number) {
    await edit({ kind: "drop-play", i });
    doc = await api.importDoc();
    if (s.plays.length < 2) overview = false;
    else await list();
  }

  async function writeAll() {
    writing = true;
    try {
      written = await api.importWriteAll(chosen.flatMap((on, i) => (on ? [i] : [])));
      error = "";
    } catch (err) {
      error = String(err);
    }
    writing = false;
  }

  function openWritten(path: string) {
    void api.importDrop();
    open(path);
  }

  /// The name of the play to start at the chosen paragraph, being asked.
  let naming = $state<string | null>(null);

  async function addPlay() {
    if (naming === null || !sel) return;
    const was = s.plays.length;
    await edit({ kind: "add-play", i: lo, title: naming });
    if (s.plays.length > was) {
      naming = null;
      doc = await api.importDoc();
      await list();
    }
  }

  async function edit(e: ImportEdit) {
    try {
      s = await api.importEdit(e);
      error = "";
    } catch (err) {
      error = String(err);
    }
    round++;
  }

  async function write() {
    try {
      open(await api.importWrite());
    } catch (err) {
      error = String(err);
    }
  }

  function drop() {
    void api.importDrop();
    close();
  }

  const PAGES: [string, string, string][] = [
    ["play", "Oyun", "Play"],
    ["cover", "Kapak", "Cover"],
    ["imprint", "Künye", "Imprint"],
    ["contents", "İçindekiler", "Contents"],
    ["preface", "Önsöz", "Preface"],
    ["author", "Yazar", "Author"],
    ["cast", "Kişiler", "Cast"],
    ["others", "Diğer kişiler", "Others in the cast"],
    ["notes", "Notlar", "Notes"],
    ["back", "Kitabın sonu", "End of book"],
    ["footnotes", "Dipnotlar", "Footnotes"],
    ["other", "Diğer", "Other"],
  ];
  const pageName = (p: string) => {
    const e = PAGES.find((x) => x[0] === p);
    return e ? l(e[1], e[2]) : p;
  };

  let doc = $state<{ text: string; page: string | null }[]>([]);
  let tab = $state<"doc" | "cast" | "play">("doc");
  /// The list being written, until it is put in.
  let castText = $state("");

  function toCast() {
    castText = s.cast_text;
    tab = "cast";
  }

  async function castPage() {
    if (!sel) return;
    await edit({ kind: "cast-page", from: lo, to: hi });
    toCast();
  }

  const putCast = () => edit({ kind: "cast-list", text: castText });

  async function nextPlay() {
    await openPlay(((s.picked ?? -1) + 1) % s.plays.length);
    if (tab === "cast") toCast();
  }
  /// The paragraphs chosen: from where it was started to where it reached.
  let sel = $state<{ a: number; b: number } | null>(null);
  /// Marking a part goes on to the next, as replacing does in a search.
  let onward = $state(true);
  let pane = $state<HTMLElement>();
  const lo = $derived(sel ? Math.min(sel.a, sel.b) : -1);
  const hi = $derived(sel ? Math.max(sel.a, sel.b) : -1);

  /// The document in runs of one label.
  const parts = $derived.by(() => {
    const out: { from: number; to: number; page: string }[] = [];
    s.labels.forEach((page, i) => {
      const last = out[out.length - 1];
      if (last && last.page === page) last.to = i;
      else out.push({ from: i, to: i, page });
    });
    return out;
  });

  function where(i: number) {
    const from = doc[i]?.page;
    const to = doc.slice(i).find((p) => p.page)?.page;
    return from ?? to ?? null;
  }

  function pagesOf(p: { from: number; to: number }) {
    const a = where(p.from);
    const b = doc[p.to]?.page;
    if (!a) return "";
    return a === b || !b ? l(`s. ${a}`, `p. ${a}`) : l(`s. ${a}–${b}`, `pp. ${a}–${b}`);
  }

  async function show(i: number) {
    tab = "doc";
    await tick();
    pane?.querySelector(`[data-i="${i}"]`)?.scrollIntoView({ block: "center" });
  }

  function pick(i: number, e: MouseEvent) {
    sel = e.shiftKey && sel ? { a: sel.a, b: i } : { a: i, b: i };
  }

  function pickPart(p: { from: number; to: number }, e: MouseEvent) {
    if (e.shiftKey && sel) sel = { a: sel.a, b: p.to >= sel.a ? p.to : p.from };
    else sel = { a: p.from, b: p.to };
    void show(p.from);
  }

  /// The part after (or before) the chosen paragraphs.
  function step(by: 1 | -1) {
    const at = by > 0 ? parts.find((p) => p.from > hi) : [...parts].reverse().find((p) => p.to < Math.max(lo, 0));
    if (!at) return;
    sel = { a: at.from, b: at.to };
    void show(at.from);
  }

  async function mark(page: string) {
    if (!sel) return;
    const end = hi;
    await edit({ kind: "mark", from: lo, to: hi, page });
    if (!onward) return;
    const next = parts.find((p) => p.from > end);
    if (next) {
      sel = { a: next.from, b: next.to };
      void show(next.from);
    }
  }

  const startHere = () => sel && edit({ kind: "start", i: lo });
  const endHere = () => sel && edit({ kind: "end", i: hi });

  onMount(async () => {
    doc = await api.importDoc();
    if (s.plays.length > 1) await list();
    const play = s.labels.indexOf("play");
    if (play > 0) {
      const p = parts.find((x) => x.from === play);
      if (p) sel = { a: p.from, b: p.from };
      void show(play);
    }
  });

  function onkey(e: KeyboardEvent) {
    if (e.key === "Escape") {
      if (naming !== null) naming = null;
      else drop();
    }
    if (e.key === "F3") {
      e.preventDefault();
      step(e.shiftKey ? -1 : 1);
    }
  }

  const name = (e: Event) => (e.currentTarget as HTMLInputElement).value;
  const file = $derived(s.file.split("/").pop());
</script>

<svelte:window onkeydown={onkey} />

<div class="import">
  <div class="head">
    <strong>{l("İçe aktar", "Import")}</strong>
    <span class="muted">{file}</span>
    {#if s.plays.length > 1 && !overview}
      <button onclick={toOverview}>{l("Cilt", "Volume")}</button>
      <button onclick={nextPlay}>{l("Sonraki oyun", "Next play")}</button>
      <label class="muted">
        {l("Bu ciltte", "In this volume")}
        <select value={s.picked} onchange={pickPlay}>
          {#each s.plays as p, i}<option value={i}>{p}</option>{/each}
        </select>
      </label>
    {/if}
    <span class="grow"></span>
    {#if ocr}
      <span class="muted">{l(`OCR ${ocr[0]}/${ocr[1]} sayfa`, `OCR ${ocr[0]}/${ocr[1]} pages`)}</span>
      <button onclick={() => api.importStop()}>{l("Durdur", "Stop")}</button>
    {:else if s.ocr}
      <button
        title={l("Sayfa görüntülerini baştan okur; işaretlediğin bölümler sıfırlanır", "Reads the page images afresh; the parts you marked start over")}
        onclick={readAgain}>{l("OCR ile yeniden oku", "Read again with OCR")}</button
      >
    {/if}
    <button onclick={drop}>{written ? l("Kapat", "Close") : l("Vazgeç", "Cancel")}</button>
    {#if overview}
      <button class="primary" disabled={!count || writing || !!written} onclick={writeAll}>
        {l(`Seçilenleri oluştur (${count})`, `Make the chosen (${count})`)}
      </button>
    {:else}
      <button class="primary" onclick={write}>{l("Oyunu oluştur", "Make the play")}</button>
    {/if}
  </div>
  {#if error}<div class="error">{error}</div>{/if}
  {#if overview}
    <div class="volume">
      {#if written}
        <h3>{l("Oluşturulanlar", "Made")}</h3>
        <ul class="made">
          {#each written as w, i (i)}
            <li>
              <b>{w.title}</b>
              {#if w.path}
                <button onclick={() => openWritten(w.path!)}>{l("Aç", "Open")}</button>
                <span class="muted">{w.path}</span>
              {:else}
                <span class="fail">{w.error}</span>
              {/if}
            </li>
          {/each}
        </ul>
      {:else}
        <p class="muted">
          {l(
            `Bu kitapta ${volume.length} oyun bulundu. Almak istediklerini seç; bakmak ve düzeltmek için oyunun adına tıkla.`,
            `${volume.length} plays were found in this book. Choose those to take; click a play's name to look it over.`,
          )}
        </p>
        <table>
          <thead>
            <tr>
              <th></th>
              <th>{l("Oyun", "Play")}</th>
              <th>{l("Sayfalar", "Pages")}</th>
              <th>{l("Replik", "Lines")}</th>
              <th>{l("Kişi", "People")}</th>
              <th></th>
            </tr>
          </thead>
          <tbody>
            {#each volume as p, i (i)}
              <tr>
                <td><input type="checkbox" aria-label={p.title} bind:checked={chosen[i]} /></td>
                <td>
                  <button class="name" onclick={() => openPlay(i)}>{p.title}</button>
                  {#if p.guess}
                    <span class="guess" title={l("İçindekilerde yok, sayfalardan tahmin edildi", "Not in the contents: a guess from the pages")}>{l("tahmin", "guess")}</span>
                  {/if}
                </td>
                <td class="num">{p.pages ? (p.pages[0] === p.pages[1] ? p.pages[0] : `${p.pages[0]}–${p.pages[1]}`) : ""}</td>
                <td class="num">{p.lines}</td>
                <td class="num">{p.people}</td>
                <td class="acts">
                  <button
                    title={l("Yanlış bulundu: metni bir öncekine katılır", "Wrongly found: its text joins the one before")}
                    onclick={() => dropPlay(i)}>{l("Oyun değil", "Not a play")}</button
                  >
                </td>
              </tr>
            {/each}
          </tbody>
        </table>
      {/if}
    </div>
  {:else}
  <div class="body">
    <div class="left">
      {#key round}
        <div class="info">
          <label>
            {l("Başlık", "Title")}
            <input value={s.title} placeholder={l("(başlıksız)", "(no title)")} onchange={(e) => edit({ kind: "info", title: name(e), author: s.author })} />
          </label>
          <label>
            {l("Yazar", "Author")}
            <input value={s.author} onchange={(e) => edit({ kind: "info", title: s.title, author: name(e) })} />
          </label>
        </div>
      {/key}
      <p class="muted">
        {l(
          `${s.lines} replik · ${s.directions} yönerge · ${s.acts} perde · ${s.scenes} sahne`,
          `${s.lines} lines · ${s.directions} directions · ${s.acts} acts · ${s.scenes} scenes`,
        )}{s.plain ? l(` · ${s.plain} düz paragraf`, ` · ${s.plain} plain paragraphs`) : ""}
      </p>
      <h3>{l("Bölümler", "Parts")}</h3>
      <div class="parts">
        {#each parts as p (p.from)}
          <button class="part {p.page}" class:on={p.from <= hi && p.to >= lo} onclick={(e) => pickPart(p, e)}>
            <b>{pageName(p.page)}</b>
            <span class="muted">{pagesOf(p)}</span>
            <span class="first">{doc[p.from]?.text.split("\n")[0] ?? ""}</span>
            <span class="muted">{p.to - p.from + 1}</span>
          </button>
        {/each}
      </div>
      <h3>{l("Kişiler", "People")}</h3>
      {#if s.people.length === 0}
        <p class="muted">{l("Hiç konuşmacı bulunamadı; metin düz paragraf olarak gelecek.", "No speakers found; the text comes in as plain paragraphs.")}</p>
      {:else}
        <table>
          <thead>
            <tr>
              <th>{l("Ad", "Name")}</th>
              <th>{l("Kısaltma", "Alias")}</th>
              <th>{l("Replik", "Lines")}</th>
              <th>{l("İlk repliği", "First line")}</th>
              <th></th>
            </tr>
          </thead>
          <tbody>
            {#key round}
              {#each s.people as p, i (i)}
                <tr class:off={p.not_character || p.merged}>
                  <td><input class="who" value={p.name} disabled={!!p.merged || p.not_character} onchange={(e) => edit({ kind: "person", i, name: name(e), alias: p.alias })} /></td>
                  <td><input class="alias" value={p.alias} disabled={!!p.merged || p.not_character} onchange={(e) => edit({ kind: "person", i, name: p.name, alias: name(e) })} /></td>
                  <td class="num">{p.lines}</td>
                  <td class="sample">{p.sample}</td>
                  <td class="acts">
                    {#if p.merged}
                      <span class="muted">→ {p.merged}</span>
                      <button onclick={() => edit({ kind: "unmerge", i })}>{l("Ayır", "Split")}</button>
                    {:else}
                      <select
                        title={l("Aynı kişi: repliklerini ona ver", "The same person: give their lines to")}
                        onchange={(e) => edit({ kind: "merge", from: i, into: Number(e.currentTarget.value) })}
                      >
                        <option value="" selected disabled>{l("Birleştir…", "Merge…")}</option>
                        {#each s.people as o, j (j)}
                          {#if j !== i && !o.merged && !o.not_character}<option value={j}>{o.name}</option>{/if}
                        {/each}
                      </select>
                      <button
                        title={l("Ad sanıldı ama değil: satırları düz metin kalır", "Caught as a name, but isn't: its lines stay text")}
                        onclick={() => edit({ kind: "not-character", i })}>{p.not_character ? l("Karakter", "Character") : l("Karakter değil", "Not a character")}</button
                      >
                    {/if}
                  </td>
                </tr>
              {/each}
            {/key}
          </tbody>
        </table>
      {/if}
    </div>
    <div class="right">
      <div class="tabs">
        <button class:on={tab === "doc"} onclick={() => (tab = "doc")}>{l("Belge", "Document")}</button>
        <button class:on={tab === "cast"} onclick={toCast}>{l("Kişi listesi", "Cast list")}</button>
        <button class:on={tab === "play"} onclick={() => (tab = "play")}>{l("Oyun", "Play")}</button>
        <span class="grow"></span>
        <button title="Shift+F3" onclick={() => step(-1)}>← {l("Önceki", "Previous")}</button>
        <button title="F3" onclick={() => step(1)}>{l("Sonraki", "Next")} →</button>
      </div>
      {#if tab === "doc"}
        <div class="tools">
          {#if sel}
            <span class="muted">{l(`${hi - lo + 1} paragraf seçili`, `${hi - lo + 1} paragraphs chosen`)}</span>
            <button class="primary" onclick={startHere}>{l("Oyun buradan başlıyor", "The play starts here")}</button>
            <button onclick={endHere}>{l("Oyun burada bitiyor", "The play ends here")}</button>
            <button
              title={l("Seçili paragraflar kişi sayfası sayılır, listesi onlardan çıkarılır", "The chosen paragraphs are the cast page, and a list is made from them")}
              onclick={castPage}>{l("Kişi sayfası bu", "This is the cast page")}</button
            >
            {#if naming === null}
              <button onclick={() => (naming = doc[lo]?.text.split("\n")[0] ?? "")}>
                {l("Burada yeni oyun başlıyor", "A new play starts here")}
              </button>
            {:else}
              <span class="row">
                <input
                  class="naming"
                  aria-label={l("Yeni oyunun adı", "The new play's name")}
                  bind:value={naming}
                  onkeydown={(e) => e.key === "Enter" && addPlay()}
                />
                <button class="primary" onclick={addPlay}>{l("Ekle", "Add")}</button>
                <button onclick={() => (naming = null)}>{l("Vazgeç", "Cancel")}</button>
              </span>
            {/if}
            <span class="row">
              {#each PAGES as [id, t, e] (id)}
                <button class="tag {id}" onclick={() => mark(id)}>{l(t, e)}</button>
              {/each}
            </span>
            <label class="onward"><input type="checkbox" bind:checked={onward} /> {l("İşaretleyince sonrakine geç", "Go on to the next after marking")}</label>
          {:else}
            <span class="muted">{l("Bir paragrafa ya da soldaki bir bölüme tıkla; Shift ile toplu seç.", "Click a paragraph or a part on the left; Shift chooses many.")}</span>
          {/if}
        </div>
        <div class="doc" bind:this={pane}>
          {#each doc as p, i (i)}
            {#if p.page && p.page !== doc[i - 1]?.page}<div class="pageno">{l(`Sayfa ${p.page}`, `Page ${p.page}`)}</div>{/if}
            <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_noninteractive_element_interactions -->
            <p data-i={i} class={s.labels[i]} class:on={i >= lo && i <= hi} onclick={(e) => pick(i, e)}>{p.text}</p>
          {/each}
        </div>
      {:else if tab === "cast"}
        <div class="castlist">
          <p class="muted">
            {l(
              "Her satırda bir kişi. Kişi sayfasını Belge'de seçip \"Kişi sayfası bu\" dersen liste oradan çıkar; burada düzelt.",
              "A person a line. Choose the cast page under Document and say \"This is the cast page\" for a list from it; put it right here.",
            )}
          </p>
          <pre class="muted syntax">{l(
              "AD | hakkında not\nLİSTEDEKİ AD = REPLİKTEKİ AD | not\n+ yalnızca toplu anılanlar\n# yorum",
              "NAME | a note on them\nNAME ON THE LIST = NAME OVER THE LINES | note\n+ those named only in a group\n# a comment",
            )}</pre>
          <textarea aria-label={l("Kişi listesi", "Cast list")} spellcheck="false" bind:value={castText}></textarea>
          <div class="castfoot">
            <button class="primary" disabled={castText === s.cast_text} onclick={putCast}>{l("Uygula", "Put it in")}</button>
            <button disabled={castText === s.cast_text} onclick={() => (castText = s.cast_text)}>{l("Geri al", "Undo")}</button>
          </div>
          {#if s.silent.length}
            <p class="muted"><b>{l("Listede ama hiç konuşmuyor", "Listed, never speaks")}:</b> {s.silent.join(", ")}</p>
          {/if}
          {#if s.missing.length}
            <p class="muted"><b>{l("Konuşuyor ama listede yok", "Speaks, not listed")}:</b> {s.missing.join(", ")}</p>
          {/if}
        </div>
      {:else}
        <div class="reading">
          <div class="column">
            {#each s.play as line, i (i)}
              <p class={line.kind}>
                {#if line.who}<b>{line.who}</b>{/if}
                {line.text}
              </p>
            {/each}
            {#if s.more}
              <p class="muted">{l(`… ve ${s.more} satır daha`, `… and ${s.more} more`)}</p>
            {/if}
          </div>
        </div>
      {/if}
    </div>
  </div>
  {/if}
</div>

<style>
  .castlist {
    flex: 1;
    min-height: 0;
    overflow: auto;
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 12px;
  }
  .castlist p {
    margin: 0;
  }
  .castlist .syntax {
    margin: 0;
    font: 12px/1.4 var(--mono);
  }
  .castlist textarea {
    flex: 1;
    min-height: 260px;
    resize: none;
    padding: 8px;
    font: 13px/1.5 var(--mono);
  }
  .castfoot {
    display: flex;
    gap: 8px;
  }
  .volume {
    flex: 1;
    min-height: 0;
    overflow: auto;
    padding: 16px 20px;
  }
  .volume table {
    width: 100%;
    max-width: 860px;
    border-collapse: collapse;
  }
  .volume th {
    text-align: left;
    font-weight: 500;
    color: var(--chrome-muted);
  }
  .volume th,
  .volume td {
    padding: 6px 8px;
    border-bottom: 1px solid var(--chrome-line);
  }
  .volume .num {
    font-variant-numeric: tabular-nums;
  }
  .volume .name {
    border: none;
    background: none;
    padding: 0;
    color: inherit;
    font: inherit;
    font-weight: 600;
    cursor: pointer;
    text-align: left;
  }
  .volume .name:hover {
    text-decoration: underline;
  }
  .guess {
    margin-left: 6px;
    padding: 0 6px;
    border: 1px solid var(--claret);
    border-radius: 3px;
    color: var(--claret);
    font-size: 11px;
  }
  .made {
    list-style: none;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .made li {
    display: flex;
    gap: 10px;
    align-items: baseline;
    flex-wrap: wrap;
  }
  .fail {
    color: var(--claret);
  }
  .naming {
    min-width: 220px;
  }
  .import {
    position: fixed;
    inset: 0;
    z-index: 30;
    display: flex;
    flex-direction: column;
    background: var(--chrome);
    color: var(--chrome-fg);
  }
  .head {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 10px 16px;
    border-bottom: 1px solid var(--chrome-line);
  }
  .head strong {
    font: 600 17px var(--serif);
    color: var(--accent-ink);
  }
  .grow {
    flex: 1;
  }
  .muted {
    color: var(--chrome-muted);
    font-size: 13px;
  }
  h3 {
    margin: 16px 0 6px;
    font: 600 14px var(--serif);
    color: var(--accent-ink);
  }
  .parts {
    max-height: 38vh;
    overflow: auto;
    border: 1px solid var(--chrome-line);
    border-radius: 4px;
  }
  .part {
    display: grid;
    grid-template-columns: 88px 74px minmax(0, 1fr) 36px;
    gap: 8px;
    width: 100%;
    padding: 4px 8px;
    border: 0;
    border-left: 3px solid var(--chrome-line);
    border-radius: 0;
    text-align: left;
    font-size: 13px;
  }
  .part.play {
    border-left-color: var(--accent-ink);
  }
  .part.on {
    background: var(--chrome-line);
  }
  .part .first {
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }
  .right {
    display: flex;
    flex-direction: column;
    min-height: 0;
    border-left: 1px solid var(--chrome-line);
  }
  .tabs,
  .tools {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px;
    padding: 6px 12px;
    border-bottom: 1px solid var(--chrome-line);
    font-size: 13px;
  }
  .tabs .on {
    color: var(--accent-ink);
    border-color: var(--claret);
  }
  .tools .row {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
    width: 100%;
  }
  .tag {
    font-size: 12px;
    padding: 2px 7px;
  }
  .tag.play {
    border-color: var(--claret);
    color: var(--accent-ink);
  }
  .onward {
    flex-direction: row;
    align-items: center;
    gap: 5px;
  }
  .doc {
    flex: 1;
    min-height: 0;
    overflow: auto;
    padding: 12px 20px;
    font: 15px/1.45 var(--serif);
    user-select: none;
  }
  .doc p {
    margin: 0 0 6px;
    padding: 2px 8px;
    border-left: 3px solid transparent;
    white-space: pre-wrap;
    color: var(--chrome-muted);
    cursor: pointer;
    content-visibility: auto;
    contain-intrinsic-size: auto 24px;
  }
  .doc p.play {
    border-left-color: var(--accent-ink);
    color: var(--chrome-fg);
  }
  .doc p.on {
    background: var(--chrome-line);
    color: var(--chrome-fg);
  }
  .pageno {
    margin: 10px 0 6px;
    font: 11px var(--sans, sans-serif);
    letter-spacing: 0.04em;
    color: var(--chrome-muted);
    border-bottom: 1px dashed var(--chrome-line);
  }
  .error {
    padding: 6px 16px;
    color: #f6efe4;
    background: var(--claret);
    font-size: 13px;
  }
  .body {
    flex: 1;
    min-height: 0;
    display: grid;
    grid-template-columns: minmax(0, 2fr) minmax(0, 3fr);
  }
  .left {
    overflow: auto;
    padding: 14px 16px;
  }
  .info {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 12px;
  }
  label {
    display: flex;
    flex-direction: column;
    gap: 3px;
    font-size: 12px;
    color: var(--chrome-muted);
  }
  input,
  select,
  button {
    border: 1px solid var(--chrome-line);
    border-radius: 4px;
    padding: 3px 7px;
    background: var(--chrome);
    color: var(--chrome-fg);
  }
  button {
    cursor: pointer;
  }
  .primary {
    background: var(--claret);
    color: #f6efe4;
    border-color: var(--claret);
  }
  table {
    width: 100%;
    border-collapse: collapse;
  }
  th,
  td {
    padding: 3px 6px;
    text-align: left;
  }
  th {
    font-size: 12px;
    font-weight: 600;
    color: var(--chrome-muted);
  }
  .who {
    width: 130px;
    font-weight: 700;
  }
  .alias {
    width: 56px;
  }
  .num {
    text-align: right;
  }
  .sample {
    max-width: 200px;
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
    color: var(--chrome-muted);
    font-size: 13px;
  }
  .acts {
    white-space: nowrap;
  }
  .acts select,
  .acts button {
    font-size: 12px;
    padding: 1px 5px;
  }
  .off .who,
  .off .alias,
  .off .sample {
    opacity: 0.5;
  }
  .reading {
    overflow: auto;
    padding: 16px 20px;
    border-left: 1px solid var(--chrome-line);
  }
  .column {
    font: 15px/1.45 var(--serif);
  }
  .column p {
    margin: 0 0 0.5em;
  }
  .column b {
    margin-right: 0.4em;
    font-size: 0.85em;
  }
  .act,
  .scene,
  .section {
    margin-top: 1em !important;
    font-weight: 700;
    color: var(--accent-ink);
  }
  .direction {
    font-style: italic;
    color: var(--chrome-muted);
  }
  .plain {
    color: var(--chrome-muted);
  }
</style>
