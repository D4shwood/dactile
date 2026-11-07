<script lang="ts">
  // The keys drawn as a keyboard, coloured by what they are for, from the
  // list both apps share (core's `keys`). Press a key or point at it.
  import { onMount } from "svelte";
  import { api, type KeysView, type KeyView } from "./api";

  let { tr, close }: { tr: boolean; close?: () => void } = $props();
  const l = (a: string, b: string) => (tr ? a : b);

  let view = $state<KeysView | null>(null);
  let cap = $state("");
  let ctrl = $state(false);
  let alt = $state(false);
  let cat = $state("");

  onMount(() => {
    void api.keys().then((k) => (view = k));
  });

  const on = (c: string): KeyView[] => view?.binds.filter((b) => b.cap === c) ?? [];
  const exact = $derived(on(cap).find((b) => b.ctrl === ctrl && b.alt === alt) ?? on(cap)[0]);
  const others = $derived(on(cap).filter((b) => b !== exact));
  const lit = (c: string) =>
    c === cap || (c === "Ctrl" && ctrl) || (c === "Alt" && alt) || (cat !== "" && on(c).some((b) => b.cat === cat));

  const NAMES: Record<string, string> = { Backspace: "⌫", Enter: "⏎", Escape: "Esc", Control: "Ctrl", "-": "−", "=": "+" };
  function onkey(e: KeyboardEvent) {
    if (e.key === "Escape" || (e.key === "F1" && !e.ctrlKey)) {
      if (!close) return;
      e.preventDefault();
      e.stopImmediatePropagation();
      return close();
    }
    if (!close && !/^F\d+$/.test(e.key) && !e.ctrlKey && !e.altKey) {
      if (document.activeElement instanceof HTMLInputElement) return;
    }
    e.preventDefault();
    e.stopImmediatePropagation();
    if (e.key === "Control" || e.key === "Alt") {
      ctrl = e.ctrlKey;
      alt = e.altKey;
      return;
    }
    cat = "";
    ctrl = e.ctrlKey;
    alt = e.altKey;
    cap = NAMES[e.key] ?? (e.key.length === 1 ? e.key.toLocaleUpperCase(tr ? "tr" : "en") : e.key);
  }
  function point(c: string) {
    cat = "";
    const first = on(c)[0];
    cap = c;
    ctrl = first?.ctrl ?? false;
    alt = first?.alt ?? false;
  }
</script>

<svelte:window onkeydown={onkey} />

{#if view}
  <div class="keyboard">
    {#each view.rows as row, i}
      <div class="row" style:padding-left={[0, 12, 24, 48, 0][i] + "px"}>
        {#each row as c}
          {@const first = on(c)[0]}
          <button
            class="cap {first?.cat ?? ''}"
            class:lit={lit(c)}
            class:mod={c === "Ctrl" || c === "Alt" || c === "Esc"}
            onmouseenter={() => point(c)}
            onfocus={() => point(c)}
            onclick={() => point(c)}>{c}</button
          >
        {/each}
      </div>
    {/each}
  </div>
  <div class="legend">
    {#each view.cats as [id, name]}
      <button class={id} class:on={cat === id} onmouseenter={() => ((cat = id), (cap = ""))} onclick={() => ((cat = id), (cap = ""))}>
        <i></i>{name}
      </button>
    {/each}
  </div>
  <div class="about">
    {#if exact}
      <p><b class={exact.cat}>{exact.combo}</b> {exact.what}</p>
      {#each others as b}<p class="other"><b>{b.combo}</b> {b.what}</p>{/each}
    {:else if cap}
      <p class="other">{l("Bu tuşun burada bir işi yok.", "This key does nothing special here.")}</p>
    {:else if cat}
      {#each view.binds.filter((b) => b.cat === cat) as b}<p class="other"><b class={b.cat}>{b.combo}</b> {b.what}</p>{/each}
    {:else}
      <p class="other">{l("Bir tuşa bas ya da fareyle üstüne gel: ne yaptığı burada yazar.", "Press a key or point at it: what it does shows here.")}</p>
    {/if}
  </div>
{/if}

<style>
  .keyboard,
  .legend,
  .about {
    --file: var(--claret, #8a3a25);
    --edit: #b9a27a;
    --panel: #6f93b8;
    --view: #86a86e;
    --write: #d39a3f;
  }
  .file { --c: #b4553a; }
  .edit { --c: #b9a27a; }
  .panel { --c: #6f93b8; }
  .view { --c: #86a86e; }
  .write { --c: #d39a3f; }
  .keyboard {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .row {
    display: flex;
    gap: 4px;
  }
  .cap {
    min-width: 40px;
    height: 36px;
    padding: 0 6px;
    font: 600 13px var(--sans, system-ui);
    color: var(--chrome-fg);
    background: none;
    border: 1px solid var(--c, var(--chrome-line));
    border-radius: 6px;
    cursor: default;
  }
  .cap:not(.file, .edit, .panel, .view, .write, .mod) {
    color: var(--chrome-muted);
    font-weight: 400;
  }
  .cap.lit {
    color: var(--chrome);
    background: var(--c, var(--chrome-muted));
  }
  .legend {
    display: flex;
    flex-wrap: wrap;
    gap: 14px;
    margin: 12px 0 8px;
  }
  .legend button {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 0;
    font-size: 13px;
    color: var(--chrome-muted);
    background: none;
    border: 0;
    cursor: default;
  }
  .legend button.on {
    color: var(--c);
    font-weight: 700;
  }
  .legend i {
    width: 10px;
    height: 10px;
    background: var(--c);
  }
  .about {
    min-height: 66px;
    max-height: 170px;
    overflow-y: auto;
    font-size: 13px;
  }
  .about p {
    margin: 0 0 3px;
  }
  .about b {
    margin-right: 6px;
    color: var(--c, inherit);
  }
  .other {
    color: var(--chrome-muted);
  }
</style>
