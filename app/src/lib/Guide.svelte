<script lang="ts">
  // Help > Commands: what a play's commands do, beside the page.
  import { onMount } from "svelte";
  import { api, type GuideGroup } from "./api";

  let { tr, close }: { tr: boolean; close: () => void } = $props();
  const l = (a: string, b: string) => (tr ? a : b);

  let groups = $state<GuideGroup[]>([]);
  let open = $state("");
  let query = $state("");
  onMount(() => {
    void api.guide().then((g) => (groups = g));
  });
  const q = $derived(query.trim().toLocaleLowerCase(tr ? "tr" : "en"));
  const shown = $derived(
    groups
      .map((g) => ({
        ...g,
        entries: g.entries.filter((e) => (e.name + " " + e.what).toLocaleLowerCase(tr ? "tr" : "en").includes(q)),
      }))
      .filter((g) => g.entries.length),
  );
</script>

<aside class="guide">
  <header>
    <h3>{l("Komutlar", "Commands")}</h3>
    <button title={l("Kapat", "Close")} onclick={close}>✕</button>
  </header>
  <input placeholder={l("Komut ara…", "Find a command…")} bind:value={query} />
  <div class="list">
    {#each shown as g}
      <h4>{g.name}</h4>
      {#each g.entries as e}
        <button class="name" class:on={open === e.name} onclick={() => (open = open === e.name ? "" : e.name)}>{e.name}</button>
        {#if open === e.name || q}
          <div class="what">
            <p>{e.what}</p>
            {#if e.example}<code>{e.example}</code>{/if}
          </div>
        {/if}
      {/each}
    {/each}
  </div>
</aside>

<style>
  .guide {
    display: flex;
    flex-direction: column;
    min-width: 0;
    height: 100%;
    padding: 10px 12px;
    color: var(--chrome-fg);
    background: var(--chrome);
    border-left: 1px solid var(--chrome-line);
    font: 13px var(--sans, system-ui);
  }
  header {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }
  h3 {
    margin: 0;
    font-size: 15px;
  }
  header button {
    color: inherit;
    background: none;
    border: 0;
    cursor: pointer;
  }
  input {
    margin: 8px 0;
    padding: 5px 7px;
    color: inherit;
    background: var(--chrome-2);
    border: 1px solid var(--chrome-line);
    border-radius: 4px;
  }
  .list {
    flex: 1;
    overflow-y: auto;
  }
  h4 {
    margin: 12px 0 4px;
    font-size: 11px;
    letter-spacing: 0.05em;
    text-transform: uppercase;
    color: var(--chrome-muted);
  }
  .name {
    display: block;
    width: 100%;
    padding: 3px 6px;
    font: 13px var(--mono, monospace);
    text-align: left;
    color: inherit;
    background: none;
    border: 0;
    border-radius: 4px;
    cursor: pointer;
  }
  .name:hover,
  .name.on {
    background: var(--chrome-2);
  }
  .what {
    padding: 4px 6px 8px;
  }
  .what p {
    margin: 0 0 4px;
    line-height: 1.35;
  }
  code {
    display: block;
    padding: 3px 5px;
    overflow-wrap: anywhere;
    font-size: 12px;
    background: var(--chrome-2);
    border-radius: 3px;
  }
</style>
