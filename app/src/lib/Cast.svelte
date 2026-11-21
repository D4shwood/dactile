<script lang="ts">
  // Ctrl+O: the people of the play beside the page, as the terminal app's
  // cast panel lists them, with how many lines and words each one has.
  import { ask } from "./asking.svelte";
  import type { CastEntry } from "./api";

  let {
    tr,
    rows,
    others,
    setOthers,
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
    /// Those under the list, who have no names.
    others: string;
    setOthers: (words: string) => void;
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

<aside class="cast ui-panel">
  <div class="ui-panel-head">
    <strong>{l("Kişiler", "Cast")}</strong>
    <span class="muted">{people.length}</span>
    <span class="grow"></span>
    <button class="ui-btn ghost icon" title={l("Yeni karakter", "New character")} onclick={add}>✚</button>
    <button class="ui-btn ghost icon" title={l("Kapat (Ctrl+O)", "Close (Ctrl+O)")} onclick={close}>✕</button>
  </div>
  <nav class="filter ui-pills">
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
          <span class="gname" title={r.name}>{r.name || l("(adsız grup)", "(unnamed group)")}</span>
          <span class="nums">{r.lines}</span>
          {#if picked === r.alias && !byLines}
            <span class="moves">
              <button class="ui-btn small" title={l("Yukarı", "Up")} onclick={(e) => (e.stopPropagation(), shift(r.alias, true))}>↑</button>
              <button class="ui-btn small" title={l("Aşağı", "Down")} onclick={(e) => (e.stopPropagation(), shift(r.alias, false))}>↓</button>
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
            <span class="name" title={r.name}>{r.star ? "★ " : ""}{r.name}</span>
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
              <button class="ui-btn small primary" onclick={(e) => (e.stopPropagation(), edit(r))}>{l("Düzenle", "Edit")}</button>
              <button class="ui-btn small" onclick={(e) => (e.stopPropagation(), find(r.alias))}>{l("İlk repliği", "First line")}</button>
              {#if !byLines}
                <button class="ui-btn small" title={l("Tabloda yukarı", "Up in the table")} onclick={(e) => (e.stopPropagation(), shift(r.alias, true))}>↑</button>
                <button class="ui-btn small" title={l("Tabloda aşağı", "Down in the table")} onclick={(e) => (e.stopPropagation(), shift(r.alias, false))}>↓</button>
              {/if}
              <button class="ui-btn small" onclick={(e) => (e.stopPropagation(), (merging = !merging), (into = ""))}>{l("Birleştir…", "Merge…")}</button>
              <button
                class="ui-btn small danger"
                onclick={async (e) => {
                  e.stopPropagation();
                  const yes = await ask({
                    title: l(`\\${r.alias} silinsin mi?`, `Delete \\${r.alias}?`),
                    detail: l("Replikleri yerinde kalır; geri almak için Ctrl+Z.", "Their lines stay; Ctrl+Z brings them back."),
                    yes: l("Sil", "Delete"),
                    no: l("Vazgeç", "Cancel"),
                    danger: true,
                    at: e,
                  });
                  if (yes) remove(r.alias);
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
                <select class="ui-field" bind:value={into} onclick={(e) => e.stopPropagation()}>
                  <option value="" disabled>…</option>
                  {#each people.filter((p) => p.alias !== r.alias) as p}
                    <option value={p.alias}>{p.name} (\{p.alias})</option>
                  {/each}
                </select>
                <button class="ui-btn small primary" disabled={!into}>{l("Birleştir", "Merge")}</button>
              </form>
            {/if}
          {/if}
        </div>
      {/if}
    {/each}
  </div>
  <label class="others ui-label">
    {l("Diğer kişiler", "Others")}
    <textarea
      class="ui-field"
      rows="2"
      value={others}
      placeholder={l("Köylüler, müzisyenler…", "Villagers, musicians…")}
      onchange={(e) => setOthers(e.currentTarget.value)}
    ></textarea>
  </label>
  <p class="foot ui-panel-foot">{l(`${total} replik · çift tık düzenler`, `${total} lines · double-click edits`)}</p>
</aside>

<style>
  .others {
    padding: 8px 10px;
    border-top: 1px solid var(--chrome-line);
  }
  .filter {
    padding: 6px 10px;
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
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .person {
    min-width: 0;
    margin: 5px 0;
    padding: 7px 10px;
    overflow-wrap: anywhere;
    background: var(--raised);
    border: 1px solid var(--chrome-line);
    border-left: 3px solid var(--accent-ink);
    border-radius: var(--r-md);
    cursor: pointer;
  }
  .person:hover {
    border-color: color-mix(in srgb, var(--chrome-fg) 25%, var(--chrome-line));
    border-left-color: var(--accent-ink);
  }
  .person.member {
    margin-left: 14px;
  }
  .picked {
    border-color: var(--accent-ink);
    box-shadow: 0 0 0 2px var(--on);
  }
  .top {
    display: flex;
    align-items: baseline;
    gap: 8px;
  }
  .name {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-weight: 700;
    letter-spacing: 0.03em;
  }
  code {
    font-family: var(--mono);
    font-size: 12px;
    color: var(--accent-ink);
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
</style>
