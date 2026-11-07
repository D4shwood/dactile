<script lang="ts">
  // The play's headings down the left of the writing, as in the terminal
  // app's margin: a click goes there, the cursor's one is marked.
  import type { Head } from "./api";

  let { tr, heads, at, go }: { tr: boolean; heads: Head[]; at: number; go: (from: number) => void } = $props();

  const here = $derived(heads.filter((h) => h.from <= at).length - 1);
  let list: HTMLElement | undefined = $state();
  $effect(() => {
    list?.querySelector(".on")?.scrollIntoView({ block: "nearest" });
  });
</script>

<nav class="scene-list" aria-label={tr ? "Sahneler" : "Scenes"} bind:this={list}>
  {#each heads as h, i (h.from)}
    <button class:top={h.depth === 0} class:on={i === here} class:hidden={h.hidden} title={h.label} onclick={() => go(h.from)}>
      {h.label}
    </button>
  {/each}
</nav>

<style>
  .scene-list {
    position: absolute;
    inset: 0 auto 0 0;
    width: 220px;
    padding: 12px 6px 40px;
    overflow-y: auto;
    background: var(--desk);
    border-right: 1px solid var(--chrome-line);
    font: 12.5px var(--sans, system-ui);
  }
  button {
    display: block;
    width: 100%;
    padding: 3px 8px 3px 20px;
    overflow: hidden;
    color: var(--faint, var(--chrome-muted));
    text-align: left;
    text-overflow: ellipsis;
    white-space: nowrap;
    background: none;
    border: 0;
    border-radius: 4px;
    cursor: pointer;
  }
  button.top {
    margin-top: 6px;
    padding-left: 8px;
    color: var(--chrome-muted);
    font-weight: 600;
  }
  button.hidden {
    font-style: italic;
  }
  button:hover {
    background: var(--chrome-2);
  }
  button.on {
    color: var(--accent-ink);
    font-weight: 600;
  }
</style>
