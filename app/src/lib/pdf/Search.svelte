<script lang="ts">
  // The search panel: as you type, over the whole PDF, with every hit marked.
  import { onDestroy } from "svelte";
  import type { PDFDocumentProxy } from "pdfjs-dist";
  import Icon from "./Icon.svelte";
  import { Finder, findIn, rectsOf, stepHit, type Hit } from "./search";
  import type { Mark, Sheets } from "./sheets";

  let {
    tr,
    doc,
    page,
    sheets,
    say,
  }: {
    tr: boolean;
    doc: PDFDocumentProxy | null;
    page: number;
    sheets: Sheets;
    say: (text: string, warn?: boolean) => void;
  } = $props();

  const l = (t: string, e: string) => (tr ? t : e);
  const LISTED = 200;

  let input: HTMLInputElement | undefined = $state();
  let query = $state("");
  let exact = $state(false);
  let words = $state(false);
  let wrap = $state(true);
  let backward = $state(false);
  let hits = $state.raw<Hit[]>([]);
  let at = $state(-1);
  let reading = $state(false);

  const finder = new Finder();
  let run = 0;
  let timer = 0;
  let asked = "";

  export function focus() {
    input?.focus();
    input?.select();
  }

  /// F3 and Shift+F3: the next hit, the search's way or against it.
  export function again(against = false) {
    const to = stepHit(at, hits.length, backward !== against, wrap);
    if (to < 0) return say(hits.length ? l("Başka sonuç yok.", "No more results.") : l("Sonuç yok.", "No results."));
    show(to);
  }

  function show(i: number) {
    at = i;
    paint();
    const h = hits[i];
    const t = finder.known(h.page);
    const r = t && rectsOf(t, h.from, h.to)[0];
    if (r) void sheets.reveal(h.page, r);
  }

  function paint() {
    const marks = new Map<number, Mark[]>();
    hits.forEach((h, i) => {
      const t = finder.known(h.page);
      if (!t) return;
      const list = marks.get(h.page) ?? [];
      for (const rect of rectsOf(t, h.from, h.to)) list.push({ rect, kind: i === at ? "now" : "hit" });
      marks.set(h.page, list);
    });
    sheets.mark(marks);
  }

  async function search(n: number, d: PDFDocumentProxy, typed: boolean) {
    const found: Hit[] = [];
    reading = true;
    try {
      for (let p = 1; p <= d.numPages; p++) {
        const t = await finder.text(d, p);
        if (n !== run) return;
        for (const [from, to] of findIn(t.text, query, { exact, words, tr })) found.push({ page: p, from, to });
        if (p % 20 === 0) hits = [...found];
      }
    } catch {
      return; // The document closed: a new one brings a new search.
    } finally {
      if (n === run) reading = false;
    }
    hits = found;
    const after = found.findIndex((h) => h.page >= page);
    const before = found.length - 1 - [...found].reverse().findIndex((h) => h.page <= page);
    at = !found.length ? -1 : backward ? (before < found.length ? before : found.length - 1) : after >= 0 ? after : 0;
    if (typed && at >= 0) show(at);
    else paint();
  }

  // Each change of the search starts it over, a moment after the last key.
  $effect(() => {
    const d = doc;
    const now = `${query}\n${exact}\n${words}`;
    clearTimeout(timer);
    const n = ++run;
    if (!d || !query) {
      hits = [];
      at = -1;
      reading = false;
      sheets.mark(new Map());
      return;
    }
    const typed = now !== asked;
    asked = now;
    timer = window.setTimeout(() => search(n, d, typed), 180);
  });

  onDestroy(() => {
    run++;
    clearTimeout(timer);
    sheets.mark(new Map());
  });

  function snippet(h: Hit) {
    const text = finder.known(h.page)?.text ?? "";
    return text.slice(Math.max(0, h.from - 18), h.to + 24).trim();
  }
</script>

<div class="search">
  <div class="line">
    <input
      bind:this={input}
      bind:value={query}
      placeholder={l("PDF'te ara", "Search the PDF")}
      title={l("PDF'te ara: Enter sonraki, Shift+Enter önceki", "Search the PDF: Enter next, Shift+Enter previous")}
      aria-label={l("PDF'te ara", "Search the PDF")}
      onkeydown={(e) => {
        if (e.key !== "Enter") return;
        e.preventDefault();
        e.stopPropagation();
        again(e.shiftKey);
      }}
    />
    <button title={l("Önceki (Shift+F3)", "Previous (Shift+F3)")} aria-label={l("Önceki", "Previous")} onclick={() => again(!backward)}><Icon name="previous" /></button>
    <button title={l("Sonraki (F3)", "Next (F3)")} aria-label={l("Sonraki", "Next")} onclick={() => again(backward)}><Icon name="next" /></button>
  </div>
  <label title={l("Büyük küçük harf ayrı", "Upper and lower case differ")}><input type="checkbox" bind:checked={exact} /> {l("Harf duyarlı", "Match case")}</label>
  <label title={l("Yalnızca bütün sözcükler", "Whole words only")}><input type="checkbox" bind:checked={words} /> {l("Bütün sözcük", "Whole words")}</label>
  <label title={l("Sona gelince başa dön", "Go round at the end")}><input type="checkbox" bind:checked={wrap} /> {l("Başa dön", "Wrap")}</label>
  <label title={l("Sondan başa doğru ara", "Search towards the start")}><input type="checkbox" bind:checked={backward} /> {l("Geriye doğru", "Backward")}</label>
  <p class="count" aria-live="polite">
    {#if reading}{l("aranıyor…", "searching…")}{:else if query}{hits.length ? `${at + 1} / ${hits.length}` : l("Sonuç yok", "No results")}{/if}
  </p>
  <div class="hits">
    {#each hits.slice(0, LISTED) as h, i (`${h.page}:${h.from}`)}
      <button class:on={i === at} title={l(`Sayfa ${h.page}`, `Page ${h.page}`)} onclick={() => show(i)}><b>{h.page}</b> {snippet(h)}</button>
    {/each}
    {#if hits.length > LISTED}<p class="count">{l(`ilk ${LISTED} sonuç gösteriliyor`, `showing the first ${LISTED}`)}</p>{/if}
  </div>
</div>

<style>
  .search {
    display: flex;
    flex-direction: column;
    gap: 4px;
    min-height: 0;
    flex: 1;
    padding: 6px;
    font-size: 12px;
  }
  .line {
    display: flex;
    gap: 2px;
  }
  .line input {
    flex: 1;
    min-width: 0;
    height: 24px;
    box-sizing: border-box;
    padding: 0 5px;
    font: inherit;
    color: inherit;
    background: var(--chrome-2);
    border: 1px solid var(--chrome-line);
    border-radius: 3px;
    caret-color: var(--claret);
  }
  .line button {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 24px;
    height: 24px;
    padding: 0;
    border: 1px solid var(--chrome-line);
    border-radius: 3px;
    background: none;
    color: inherit;
    cursor: pointer;
  }
  label {
    display: flex;
    align-items: center;
    gap: 4px;
  }
  .count {
    margin: 2px 0;
    min-height: 1.2em;
    color: var(--chrome-muted);
  }
  .hits {
    flex: 1;
    min-height: 0;
    overflow: auto;
  }
  .hits button {
    display: block;
    width: 100%;
    padding: 3px 4px;
    border: 0;
    border-radius: 3px;
    text-align: left;
    font: inherit;
    color: inherit;
    background: none;
    cursor: pointer;
    overflow-wrap: anywhere;
  }
  .hits button:hover,
  .hits button.on {
    background: var(--chrome-2);
  }
  button:focus-visible,
  input:focus-visible {
    outline: 2px solid var(--claret);
    outline-offset: -1px;
  }
</style>
