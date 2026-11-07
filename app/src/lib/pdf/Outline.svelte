<script lang="ts" module>
  export interface Entry {
    title: string;
    dest: unknown;
    url?: string | null;
    items: Entry[];
  }
</script>

<script lang="ts">
  // The PDF's bookmarks, as a tree.
  import Outline from "./Outline.svelte";

  let {
    entries,
    tr,
    follow,
  }: {
    entries: Entry[];
    tr: boolean;
    follow: (dest: unknown, url?: string) => void;
  } = $props();

  const l = (t: string, e: string) => (tr ? t : e);
  let shut = $state<Record<number, boolean>>({});
</script>

<ul>
  {#each entries as e, i (i)}
    <li>
      {#if e.items.length}
        <button class="fold" title={shut[i] ? l("Aç", "Open") : l("Kapat", "Fold")} aria-expanded={!shut[i]} onclick={() => (shut[i] = !shut[i])}>{shut[i] ? "▸" : "▾"}</button>
      {/if}
      <button class="go" title={e.title} onclick={() => follow(e.dest, e.url ?? undefined)}>{e.title}</button>
      {#if e.items.length && !shut[i]}
        <Outline entries={e.items} {tr} {follow} />
      {/if}
    </li>
  {/each}
</ul>

<style>
  ul {
    margin: 0;
    padding: 2px 0 2px 10px;
    list-style: none;
  }
  button {
    padding: 2px;
    font: inherit;
    color: inherit;
    text-align: left;
    background: none;
    border: 0;
    border-radius: 3px;
    cursor: pointer;
  }
  .go:hover {
    background: var(--chrome-2);
  }
  button:focus-visible {
    outline: 2px solid var(--accent-ink);
  }
</style>
