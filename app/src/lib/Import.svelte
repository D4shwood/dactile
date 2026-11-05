<script lang="ts">
  // The preview of an imported document: the names found to put right, and
  // the play as it was read, before anything is written.
  import { untrack } from "svelte";
  import { api, type ImportEdit, type Shown } from "./api";

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

  const name = (e: Event) => (e.currentTarget as HTMLInputElement).value;
  const file = $derived(s.file.split("/").pop());
</script>

<svelte:window onkeydown={(e) => e.key === "Escape" && drop()} />

<div class="import">
  <div class="head">
    <strong>{l("İçe aktar", "Import")}</strong>
    <span class="muted">{file}</span>
    <span class="grow"></span>
    <button onclick={drop}>{l("Vazgeç", "Cancel")}</button>
    <button class="primary" onclick={write}>{l("Oyunu oluştur", "Make the play")}</button>
  </div>
  {#if error}<div class="error">{error}</div>{/if}
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
      {#if s.asides.length}
        <h3>{l("Kitabın sayfaları", "The book's pages")}</h3>
        <p class="muted">{l("Bunlar oyun olarak okunmadı. İşaretli olanlar düz metin olarak oyuna girer.", "These weren't read as the play. Those ticked go into it as plain text.")}</p>
        {#each s.asides as a, i (i)}
          <details class="aside">
            <summary>
              <input type="checkbox" checked={a.keep} aria-label={l("Oyuna al", "Take into the play")} onclick={(e) => e.stopPropagation()} onchange={() => edit({ kind: "aside", i })} />
              {a.label}
              <span class="muted">{l(`${a.paragraphs} paragraf`, `${a.paragraphs} paragraphs`)}</span>
            </summary>
            <p>{a.text}</p>
          </details>
        {/each}
      {/if}
    </div>
    <div class="play">
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
  </div>
</div>

<style>
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
    color: var(--claret);
  }
  .grow {
    flex: 1;
  }
  .muted {
    color: var(--chrome-muted);
    font-size: 13px;
  }
  h3 {
    margin: 18px 0 2px;
    font: 600 14px var(--serif);
    color: var(--claret);
  }
  .aside {
    padding: 3px 0;
    border-top: 1px solid var(--chrome-line);
    font-size: 13px;
  }
  .aside summary {
    cursor: pointer;
  }
  .aside p {
    margin: 6px 0 8px 22px;
    max-height: 180px;
    overflow: auto;
    white-space: pre-wrap;
    color: var(--chrome-muted);
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
    grid-template-columns: minmax(0, 3fr) minmax(0, 2fr);
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
  .play {
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
    color: var(--claret);
  }
  .direction {
    font-style: italic;
    color: var(--chrome-muted);
  }
  .plain {
    color: var(--chrome-muted);
  }
</style>
