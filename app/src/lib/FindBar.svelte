<script lang="ts">
  import {
    closeSearchPanel,
    findNext,
    findPrevious,
    replaceAll,
    replaceNext,
    setSearchQuery,
  } from "@codemirror/search";
  import { EditorView } from "@codemirror/view";
  import { countOf, findLang, hitsOf, queryOf, type FindOptions, type Hit } from "./find";
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
  let hits = $state<Hit[]>([]);
  /// Where the window stands, once it has been dragged or opened.
  let at = $state({ x: 0, y: 0 });

  function update() {
    const q = queryOf(o);
    view.dispatch({ effects: setSearchQuery.of(q) });
    count = o.text ? countOf(view, q) : { all: 0, at: 0 };
    hits = advanced && o.text ? hitsOf(view, q) : [];
  }
  function openAdvanced() {
    advanced = true;
    const r = view.dom.closest(".left")?.getBoundingClientRect();
    at = { x: Math.max(8, (r?.right ?? innerWidth) - 500), y: (r?.top ?? 100) + 56 };
    update();
  }
  function show(h: Hit) {
    view.dispatch({
      selection: { anchor: h.from, head: h.to },
      effects: EditorView.scrollIntoView(h.from, { y: "center" }),
    });
    update();
  }
  function drag(e: PointerEvent) {
    if ((e.target as HTMLElement).closest("button")) return;
    const head = e.currentTarget as HTMLElement;
    const from = { ...at, px: e.clientX, py: e.clientY };
    head.setPointerCapture(e.pointerId);
    const move = (m: PointerEvent) => {
      at = {
        x: Math.min(innerWidth - 80, Math.max(-380, from.x + m.clientX - from.px)),
        y: Math.min(innerHeight - 40, Math.max(0, from.y + m.clientY - from.py)),
      };
    };
    const up = () => {
      head.removeEventListener("pointermove", move);
      head.removeEventListener("pointerup", up);
    };
    head.addEventListener("pointermove", move);
    head.addEventListener("pointerup", up);
    e.preventDefault();
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
  <button class="more" onclick={openAdvanced}>{l("Gelişmiş", "Advanced")}</button>
  <button title={l("Kapat (Esc)", "Close (Esc)")} onclick={close}>×</button>
</div>

{#if advanced}
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="window" style:left="{at.x}px" style:top="{at.y}px" onkeydown={keys}>
    <header onpointerdown={drag} title={l("Sürükle", "Drag")}>
      <h3>{l("Bul ve değiştir", "Find and replace")}</h3>
      <button title={l("Kapat (Esc)", "Close (Esc)")} onclick={() => (advanced = false)}>×</button>
    </header>
    <div class="fields">
      <label>
        {l("Aranan", "Find")}
        <!-- svelte-ignore a11y_autofocus -->
        <input bind:value={o.text} oninput={update} autofocus />
      </label>
      <label>
        {l("Yerine", "Replace with")}
        <input bind:value={o.replace} oninput={update} />
      </label>
    </div>
    <div class="ticks">
      <label><input type="checkbox" bind:checked={o.caseSensitive} onchange={update} /> {l("Büyük/küçük harf", "Match case")}</label>
      <label><input type="checkbox" bind:checked={o.wholeWord} onchange={update} /> {l("Tam sözcük", "Whole words")}</label>
      <label><input type="checkbox" bind:checked={o.regexp} onchange={update} /> {l("Düzenli ifade", "Regular expression")}</label>
    </div>
    <div class="row">
      <button type="button" onclick={() => go(findPrevious)}>{l("Önceki", "Previous")}</button>
      <button type="button" onclick={() => go(findNext)}>{l("Sonraki", "Next")}</button>
      <span class="grow"></span>
      <button type="button" disabled={view.state.readOnly} onclick={() => go(replaceNext)}>{l("Değiştir", "Replace")}</button>
      <button type="button" disabled={view.state.readOnly} onclick={() => go(replaceAll)}>{l("Tümünü değiştir", "Replace all")}</button>
    </div>
    <p class="found">
      {#if o.text}
        {count.all > hits.length ? l(`${count.all} sonuç, ilk ${hits.length} tanesi:`, `${count.all} found, the first ${hits.length}:`) : l(`${count.all} sonuç`, `${count.all} found`)}
      {/if}
    </p>
    {#if hits.length}
      <ul class="hits">
        {#each hits as h (h.from)}
          <li>
            <button class:on={view.state.selection.main.from === h.from} onclick={() => show(h)}>
              <small>{h.line}</small>
              <span>{h.before}<mark>{h.match}</mark>{h.after}</span>
            </button>
          </li>
        {/each}
      </ul>
    {/if}
  </div>
{/if}

<style>
  .bar {
    position: absolute;
    top: 8px;
    right: 22px;
    z-index: 20;
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
  .window {
    position: fixed;
    z-index: 50;
    display: flex;
    flex-direction: column;
    gap: 10px;
    width: min(480px, calc(100vw - 16px));
    max-height: calc(100vh - 120px);
    padding: 0 14px 14px;
    font: 13px var(--sans, system-ui);
    color: var(--chrome-fg);
    background: var(--chrome);
    border: 1px solid var(--chrome-line);
    border-radius: 8px;
    box-shadow: 0 10px 30px rgb(0 0 0 / 0.3);
  }
  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin: 0 -14px;
    padding: 8px 10px 8px 14px;
    border-bottom: 1px solid var(--chrome-line);
    cursor: move;
    touch-action: none;
    user-select: none;
  }
  h3 {
    margin: 0;
    font-size: 14px;
  }
  .fields {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 10px;
  }
  label {
    display: grid;
    gap: 4px;
  }
  .ticks {
    display: flex;
    flex-wrap: wrap;
    gap: 6px 14px;
  }
  .ticks label {
    display: flex;
    gap: 6px;
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
  }
  .row button {
    border-color: var(--chrome-line);
  }
  .grow {
    flex: 1;
  }
  .hits {
    flex: 1;
    min-height: 60px;
    margin: 0;
    padding: 0;
    overflow-y: auto;
    list-style: none;
  }
  .hits button {
    display: flex;
    gap: 8px;
    width: 100%;
    padding: 3px 6px;
    overflow: hidden;
    text-align: left;
    white-space: nowrap;
  }
  .hits button.on {
    background: var(--chrome-2);
  }
  .hits small {
    min-width: 30px;
    text-align: right;
    font-variant-numeric: tabular-nums;
    opacity: 0.6;
  }
  .hits span {
    overflow: hidden;
    text-overflow: ellipsis;
  }
  mark {
    color: inherit;
    background: color-mix(in srgb, var(--accent-ink, #c96) 40%, transparent);
  }
</style>
