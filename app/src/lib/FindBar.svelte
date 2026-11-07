<script lang="ts">
  import {
    closeSearchPanel,
    findNext,
    findPrevious,
    replaceAll,
    replaceNext,
    setSearchQuery,
  } from "@codemirror/search";
  import type { EditorView } from "@codemirror/view";
  import { countOf, findLang, queryOf, type FindOptions } from "./find";
  import { onMount } from "svelte";

  let { view }: { view: EditorView } = $props();

  const l = (t: string, e: string) => (findLang.tr ? t : e);
  // svelte-ignore state_referenced_locally
  const sel = view.state.selection.main;
  // svelte-ignore state_referenced_locally
  const picked = sel.empty ? "" : view.state.sliceDoc(sel.from, sel.to);

  let o = $state<FindOptions>({
    text: picked.includes("\n") ? "" : picked,
    replace: "",
    caseSensitive: false,
    wholeWord: false,
    regexp: false,
  });
  let advanced = $state(false);
  let count = $state({ all: 0, at: 0 });
  let box: HTMLInputElement | undefined = $state();

  function update() {
    const q = queryOf(o);
    view.dispatch({ effects: setSearchQuery.of(q) });
    count = o.text ? countOf(view, q) : { all: 0, at: 0 };
  }
  function go(cmd: (v: EditorView) => boolean) {
    update();
    if (!o.text) return;
    cmd(view);
    update();
  }
  function close() {
    closeSearchPanel(view);
    view.focus();
  }
  function keys(e: KeyboardEvent) {
    e.stopPropagation();
    if (e.key === "Escape") {
      e.preventDefault();
      if (advanced) advanced = false;
      else close();
    } else if (e.key === "Enter") {
      e.preventDefault();
      go(e.shiftKey ? findPrevious : findNext);
    }
  }
  onMount(() => {
    update();
    box?.focus();
    box?.select();
  });
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="bar" onkeydown={keys}>
  <input bind:this={box} bind:value={o.text} oninput={update} placeholder={l("Bul", "Find")} aria-label={l("Bul", "Find")} />
  <span class="count">{o.text ? `${count.at}/${count.all}` : ""}</span>
  <button title={l("Önceki (Shift+Enter)", "Previous (Shift+Enter)")} onclick={() => go(findPrevious)}>↑</button>
  <button title={l("Sonraki (Enter)", "Next (Enter)")} onclick={() => go(findNext)}>↓</button>
  <button class="more" onclick={() => (advanced = true)}>{l("Gelişmiş", "Advanced")}</button>
  <button title={l("Kapat (Esc)", "Close (Esc)")} onclick={close}>×</button>
</div>

{#if advanced}
  <div class="veil" role="presentation" onmousedown={(e) => e.target === e.currentTarget && (advanced = false)}>
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div class="dialog" onkeydown={keys}>
      <h3>{l("Bul ve değiştir", "Find and replace")}</h3>
      <label>
        {l("Aranan", "Find")}
        <!-- svelte-ignore a11y_autofocus -->
        <input bind:value={o.text} oninput={update} autofocus />
      </label>
      <label>
        {l("Yerine", "Replace with")}
        <input bind:value={o.replace} oninput={update} />
      </label>
      <label class="tick"><input type="checkbox" bind:checked={o.caseSensitive} onchange={update} /> {l("Büyük/küçük harf duyarlı", "Match case")}</label>
      <label class="tick"><input type="checkbox" bind:checked={o.wholeWord} onchange={update} /> {l("Yalnızca tam sözcük", "Whole words only")}</label>
      <label class="tick"><input type="checkbox" bind:checked={o.regexp} onchange={update} /> {l("Düzenli ifade", "Regular expression")}</label>
      <p class="found">{o.text ? l(`${count.all} sonuç`, `${count.all} found`) : ""}</p>
      <div class="row">
        <button type="button" onclick={() => go(findPrevious)}>{l("Önceki", "Previous")}</button>
        <button type="button" onclick={() => go(findNext)}>{l("Sonraki", "Next")}</button>
        <button type="button" disabled={view.state.readOnly} onclick={() => go(replaceNext)}>{l("Değiştir", "Replace")}</button>
        <button type="button" disabled={view.state.readOnly} onclick={() => go(replaceAll)}>{l("Tümünü değiştir", "Replace all")}</button>
        <button type="button" class="plain" onclick={() => (advanced = false)}>{l("Kapat", "Close")}</button>
      </div>
    </div>
  </div>
{/if}

<style>
  .bar {
    position: absolute;
    top: 14px;
    right: 16px;
    z-index: 5;
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 5px 6px;
    font: 13px var(--sans, system-ui);
    color: var(--chrome-fg);
    background: var(--chrome);
    border: 1px solid var(--chrome-line);
    border-radius: 6px;
    box-shadow: 0 2px 10px rgb(0 0 0 / 0.18);
  }
  input:not([type="checkbox"]) {
    font: inherit;
    color: var(--chrome-fg);
    background: var(--chrome-2);
    border: 1px solid var(--chrome-line);
    border-radius: 4px;
    padding: 3px 6px;
  }
  .bar input {
    width: 190px;
  }
  .count {
    min-width: 42px;
    text-align: right;
    font-variant-numeric: tabular-nums;
    opacity: 0.7;
  }
  button {
    font: inherit;
    color: inherit;
    background: none;
    border: 1px solid transparent;
    border-radius: 4px;
    padding: 2px 7px;
    cursor: pointer;
  }
  button:hover:not(:disabled) {
    border-color: var(--chrome-line);
  }
  button:disabled {
    opacity: 0.45;
    cursor: default;
  }
  .veil {
    position: fixed;
    inset: 0;
    z-index: 50;
    display: grid;
    place-items: center;
    background: rgb(0 0 0 / 0.3);
  }
  .dialog {
    display: grid;
    gap: 10px;
    width: min(440px, calc(100vw - 32px));
    padding: 18px 20px;
    font: 13px var(--sans, system-ui);
    color: var(--chrome-fg);
    background: var(--chrome);
    border: 1px solid var(--chrome-line);
    border-radius: 8px;
  }
  h3 {
    margin: 0;
    font-size: 15px;
  }
  label {
    display: grid;
    gap: 4px;
  }
  label.tick {
    display: flex;
    gap: 8px;
    align-items: center;
  }
  .found {
    margin: 0;
    min-height: 1.2em;
    opacity: 0.7;
  }
  .row {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    justify-content: flex-end;
  }
  .row button {
    border-color: var(--chrome-line);
  }
  .row .plain {
    border-color: transparent;
  }
</style>
