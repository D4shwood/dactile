<script lang="ts">
  // Learning a role by heart: the cues before the role's line, the line
  // itself hidden; Space shows its first letters, then all of it, then goes
  // on to the next.
  import { onMount, tick, untrack } from "svelte";
  import type { Memo } from "./api";

  let { tr, memo: start, close }: { tr: boolean; memo: Memo; close: () => void } = $props();

  // Read once: the role doesn't change under it, and the parent lets go
  // of it on closing.
  const memo = untrack(() => start);

  const l = (t: string, e: string) => (tr ? t : e);
  /// The line being asked, an index into the cues.
  let at = $state(memo.at);
  /// 0 hidden, 1 first letters, 2 all of it.
  let shown = $state(0);
  let root: HTMLElement;
  let list = $state<HTMLElement>();

  const ours = $derived(memo.cues.flatMap((c, i) => (c.ours ? [i] : [])));
  const k = $derived(ours.indexOf(at));
  /// Enough of what comes before it to find the way in.
  const from = $derived(Math.max(0, at - 40));
  let ended = $state(false);

  function to(i: number) {
    at = i;
    shown = 0;
    ended = false;
  }

  function onkey(e: KeyboardEvent) {
    const next = ours[k + 1];
    switch (e.key) {
      case " ":
      case "Enter":
        if (shown < 2) shown++;
        else if (next !== undefined) to(next);
        else ended = true;
        break;
      case "ArrowDown":
      case "ArrowRight":
        if (next !== undefined) to(next);
        break;
      case "Backspace":
      case "ArrowUp":
      case "ArrowLeft":
        if (k > 0) to(ours[k - 1]);
        break;
      default:
        return;
    }
    e.preventDefault();
  }

  // The asked line stays at the bottom, the cues above it.
  $effect(() => {
    void at;
    void shown;
    void tick().then(() => list?.scrollTo({ top: list.scrollHeight }));
  });

  onMount(() => root.focus());
</script>

<svelte:window onkeydown={onkey} />

<div class="memo" tabindex="-1" bind:this={root}>
  <div class="head">
    <strong>{l("Ezber", "Memorize")}</strong>
    <span>{memo.who}</span>
    {#if ours.length}<span class="muted">{k + 1}/{ours.length}</span>{/if}
    <span class="grow"></span>
    <button onclick={close}>{l("Kapat", "Close")} · Esc</button>
  </div>
  {#if ours.length === 0}
    <p class="empty">{l("Bu rolün repliği yok.", "This role has no lines.")}</p>
  {:else}
    <div class="cues" bind:this={list}>
      <div class="column">
        {#each memo.cues.slice(from, at + 1) as c, i (from + i)}
          {@const asked = from + i === at}
          <p class:heading={c.heading} class:ours={c.ours} class:asked class:direction={!c.who && !c.heading}>
            {#if c.who}<b>{c.who}</b>{/if}
            {#if asked}{shown === 0 ? "…" : shown === 1 ? c.hint : c.text}{:else}{c.text}{/if}
          </p>
        {/each}
      </div>
    </div>
  {/if}
  <div class="keys">
    {#if ended}
      {l("Rolün sonu. Esc kapatır.", "The end of the role. Esc closes.")}
    {:else if shown === 0}
      {l("Boşluk ilk harfler · ⌫ önceki · ↓ atla · Esc kapat", "Space first letters · ⌫ back · ↓ skip · Esc close")}
    {:else if shown === 1}
      {l("Boşluk hepsini göster · ⌫ önceki · ↓ atla · Esc kapat", "Space show it all · ⌫ back · ↓ skip · Esc close")}
    {:else}
      {l("Boşluk sıradaki · ⌫ önceki · Esc kapat", "Space next · ⌫ back · Esc close")}
    {/if}
  </div>
</div>

<style>
  .memo {
    display: flex;
    flex-direction: column;
    height: 100%;
    background: var(--chrome);
    color: var(--chrome-fg);
    outline: none;
  }
  .head {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 8px 14px;
    border-bottom: 1px solid var(--chrome-line);
  }
  .head strong {
    font: 600 15px var(--serif);
    color: var(--accent-ink);
  }
  .muted {
    color: var(--chrome-muted);
    font-size: 12px;
  }
  .grow {
    flex: 1;
  }
  button {
    border: 1px solid var(--chrome-line);
    background: var(--chrome);
    color: var(--chrome-fg);
    border-radius: 4px;
    padding: 3px 10px;
    cursor: pointer;
  }
  .empty {
    padding: 20px;
    color: var(--chrome-muted);
  }
  .cues {
    flex: 1;
    min-height: 0;
    overflow: auto;
    padding: 20px;
  }
  .column {
    max-width: 44em;
    margin: 0 auto;
    font: 17px/1.5 var(--serif);
  }
  p {
    margin: 0 0 0.6em;
  }
  p b {
    margin-right: 0.5em;
    font-size: 0.85em;
    letter-spacing: 0.03em;
  }
  .heading {
    margin-top: 1em;
    font-weight: 700;
    color: var(--accent-ink);
  }
  .direction {
    font-style: italic;
    color: var(--chrome-muted);
  }
  .ours {
    color: var(--chrome-muted);
  }
  .asked {
    padding: 0.4em 0.6em;
    border-left: 3px solid var(--accent-ink);
    color: var(--chrome-fg);
    font-weight: 600;
  }
  .keys {
    padding: 6px 14px;
    border-top: 1px solid var(--chrome-line);
    font-size: 12px;
    color: var(--chrome-muted);
  }
</style>
