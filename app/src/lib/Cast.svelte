<script lang="ts">
  // Ctrl+O: the people of the play beside the page, as the terminal app's
  // cast panel lists them, with how many lines and words each one has.
  import type { CastEntry } from "./api";

  let {
    tr,
    rows,
    byLines = $bindable(),
    add,
    edit,
    remove,
    merge,
    shift,
    find,
    close,
  }: {
    tr: boolean;
    rows: CastEntry[];
    byLines: boolean;
    add: () => void;
    edit: (r: CastEntry) => void;
    remove: (alias: string) => void;
    merge: (from: string, into: string) => void;
    shift: (alias: string, up: boolean) => void;
    /// The character's first line in the play.
    find: (alias: string) => void;
    close: () => void;
  } = $props();

  const l = (t: string, e: string) => (tr ? t : e);
  let picked = $state<string | null>(null);
  let merging = $state(false);
  let into = $state("");

  const people = $derived(rows.filter((r) => !r.group));
  const total = $derived(people.reduce((s, r) => s + r.lines, 0));
  const kinds: Record<string, [string, string]> = {
    plain: ["tabloda yok", "not in the table"],
    hidden: ["gizli", "hidden"],
  };

  function pick(r: CastEntry) {
    if (picked !== r.alias) merging = false;
    picked = r.alias;
  }
</script>

<aside class="cast">
  <div class="head">
    <strong>{l("Kişiler", "Cast")}</strong>
    <span class="muted">{people.length}</span>
    <span class="grow"></span>
    <button title={l("Yeni karakter", "New character")} onclick={add}>✚</button>
    <button title={l("Kapat (Ctrl+O)", "Close (Ctrl+O)")} onclick={close}>✕</button>
  </div>
  <nav class="filter">
    <button class:on={!byLines} onclick={() => (byLines = false)}>{l("Tablo sırası", "Table order")}</button>
    <button class:on={byLines} onclick={() => (byLines = true)}>{l("En çok konuşan", "Most lines")}</button>
  </nav>

  <div class="list">
    {#if !rows.length}
      <p class="empty">{l("Henüz karakter yok. ✚ ile ekle, ya da yazarken \\ ile.", "No characters yet. Add one with ✚, or with \\ while writing.")}</p>
    {/if}
    {#each rows as r (r.alias)}
      {#if r.group}
        <div class="group" class:picked={picked === r.alias} role="button" tabindex="0" onclick={() => pick(r)} onkeydown={(e) => e.key === "Enter" && pick(r)}>
          <span class="gname">{r.name || l("(adsız grup)", "(unnamed group)")}</span>
          <span class="nums">{r.lines}</span>
          {#if picked === r.alias && !byLines}
            <span class="moves">
              <button title={l("Yukarı", "Up")} onclick={(e) => (e.stopPropagation(), shift(r.alias, true))}>↑</button>
              <button title={l("Aşağı", "Down")} onclick={(e) => (e.stopPropagation(), shift(r.alias, false))}>↓</button>
            </span>
          {/if}
        </div>
      {:else}
        <div
          class="person"
          class:member={r.kind === "member" && !byLines}
          class:picked={picked === r.alias}
          role="button"
          tabindex="0"
          onclick={() => pick(r)}
          ondblclick={() => edit(r)}
          onkeydown={(e) => e.key === "Enter" && edit(r)}
        >
          <div class="top">
            <span class="name">{r.star ? "★ " : ""}{r.name}</span>
            <code>\{r.alias}</code>
            <span class="grow"></span>
            <span class="nums" title={l(`${r.lines} replik, ${r.words} söz`, `${r.lines} lines, ${r.words} words`)}>
              {r.lines}<small>&nbsp;· {r.words}</small>
            </span>
          </div>
          {#if r.desc || r.full || kinds[r.kind]}
            <div class="desc">
              {[r.full, r.desc].filter(Boolean).join(" — ")}{#if kinds[r.kind]}<em> ({l(...kinds[r.kind])})</em>{/if}
            </div>
          {/if}
          {#if total}
            <div class="bar"><span style:width={`${(100 * r.lines) / Math.max(...people.map((p) => p.lines), 1)}%`}></span></div>
          {/if}
          {#if picked === r.alias}
            <div class="actions">
              <button class="primary" onclick={(e) => (e.stopPropagation(), edit(r))}>{l("Düzenle", "Edit")}</button>
              <button onclick={(e) => (e.stopPropagation(), find(r.alias))}>{l("İlk repliği", "First line")}</button>
              {#if !byLines}
                <button title={l("Tabloda yukarı", "Up in the table")} onclick={(e) => (e.stopPropagation(), shift(r.alias, true))}>↑</button>
                <button title={l("Tabloda aşağı", "Down in the table")} onclick={(e) => (e.stopPropagation(), shift(r.alias, false))}>↓</button>
              {/if}
              <button onclick={(e) => (e.stopPropagation(), (merging = !merging), (into = ""))}>{l("Birleştir…", "Merge…")}</button>
              <button
                class="danger"
                onclick={(e) => {
                  e.stopPropagation();
                  if (confirm(l(`\\${r.alias} silinsin mi?`, `Delete \\${r.alias}?`))) remove(r.alias);
                }}>{l("Sil", "Delete")}</button
              >
            </div>
            {#if merging}
              <form
                class="merge"
                onsubmit={(e) => {
                  e.preventDefault();
                  if (into) merge(r.alias, into);
                  merging = false;
                }}
              >
                <span>{l(`\\${r.alias} şununla birleşsin:`, `Merge \\${r.alias} into:`)}</span>
                <select bind:value={into} onclick={(e) => e.stopPropagation()}>
                  <option value="" disabled>…</option>
                  {#each people.filter((p) => p.alias !== r.alias) as p}
                    <option value={p.alias}>{p.name} (\{p.alias})</option>
                  {/each}
                </select>
                <button class="primary" disabled={!into}>{l("Birleştir", "Merge")}</button>
              </form>
            {/if}
          {/if}
        </div>
      {/if}
    {/each}
  </div>
  <p class="foot">{l(`${total} replik · çift tık düzenler`, `${total} lines · double-click edits`)}</p>
</aside>

<style>
  .cast {
    display: flex;
    flex-direction: column;
    min-width: 0;
    min-height: 0;
    background: var(--chrome);
    border-left: 1px solid var(--chrome-line);
  }
  .head {
    display: flex;
    gap: 6px;
    align-items: center;
    padding: 6px 10px;
    border-bottom: 1px solid var(--chrome-line);
  }
  .head button {
    width: 28px;
    height: 24px;
    padding: 0;
  }
  .grow {
    flex: 1;
  }
  .muted,
  .empty,
  .foot {
    color: var(--chrome-muted);
    font-size: 12px;
  }
  .foot {
    margin: 0;
    padding: 6px 10px;
    border-top: 1px solid var(--chrome-line);
  }
  .filter {
    display: flex;
    gap: 4px;
    padding: 6px 10px;
  }
  .filter button {
    border: 0;
    background: none;
    padding: 3px 8px;
    border-radius: 10px;
    color: var(--chrome-muted);
  }
  .filter button.on {
    background: var(--claret);
    color: #f6efe4;
  }
  .list {
    flex: 1;
    overflow: auto;
    padding: 4px 10px 16px;
  }
  .group {
    display: flex;
    align-items: center;
    gap: 6px;
    margin: 12px 0 2px;
    padding: 2px 4px;
    font-size: 11px;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--chrome-muted);
    cursor: pointer;
  }
  .group .gname {
    flex: 1;
  }
  .person {
    padding: 7px 10px;
    margin: 5px 0;
    border-radius: 6px;
    background: var(--chrome-2);
    border-left: 3px solid var(--claret);
    cursor: pointer;
  }
  .person.member {
    margin-left: 14px;
  }
  .picked {
    outline: 2px solid var(--claret);
  }
  .top {
    display: flex;
    align-items: baseline;
    gap: 8px;
  }
  .name {
    font-weight: 700;
    letter-spacing: 0.03em;
  }
  code {
    font-family: var(--mono);
    font-size: 12px;
    color: var(--claret);
  }
  .nums {
    font-variant-numeric: tabular-nums;
    font-size: 13px;
  }
  .nums small {
    color: var(--chrome-muted);
  }
  .desc {
    margin-top: 2px;
    font-family: var(--serif);
    font-size: 13.5px;
  }
  .desc em {
    color: var(--chrome-muted);
  }
  .bar {
    height: 3px;
    margin-top: 6px;
    background: var(--chrome-line);
    border-radius: 2px;
  }
  .bar span {
    display: block;
    height: 100%;
    background: var(--claret);
    border-radius: 2px;
  }
  .actions,
  .moves {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    margin-top: 8px;
  }
  .moves {
    margin: 0;
  }
  .merge {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px;
    margin-top: 8px;
    font-size: 12.5px;
  }
  select {
    border: 1px solid var(--chrome-line);
    border-radius: 4px;
    padding: 3px 6px;
    background: var(--chrome);
    color: var(--chrome-fg);
  }
  button {
    border: 1px solid var(--chrome-line);
    background: var(--chrome);
    border-radius: 4px;
    padding: 3px 9px;
    cursor: pointer;
    font-size: 12px;
    color: var(--chrome-fg);
  }
  .primary {
    background: var(--claret);
    color: #f6efe4;
    border-color: var(--claret);
  }
  .primary:disabled {
    opacity: 0.5;
  }
  .danger {
    color: var(--claret);
  }
</style>
