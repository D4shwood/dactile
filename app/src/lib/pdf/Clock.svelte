<script lang="ts">
  // The time, and how long since it was started: for a reading or a talk.
  import { onDestroy } from "svelte";

  let { tr }: { tr: boolean } = $props();
  const l = (t: string, e: string) => (tr ? t : e);

  let now = $state(Date.now());
  let start = $state(Date.now());
  const tick = setInterval(() => (now = Date.now()), 1000);
  onDestroy(() => clearInterval(tick));

  const two = (n: number) => String(n).padStart(2, "0");
  const time = $derived(new Date(now).toLocaleTimeString(tr ? "tr-TR" : "en-GB", { hour: "2-digit", minute: "2-digit" }));
  const gone = $derived(Math.max(0, Math.floor((now - start) / 1000)));
</script>

<div class="clock">
  <strong>{time}</strong>
  <span>{two(Math.floor(gone / 3600))}:{two(Math.floor(gone / 60) % 60)}:{two(gone % 60)}</span>
  <button title={l("Süreyi sıfırla", "Reset the timer")} onclick={() => (start = now = Date.now())}>{l("Sıfırla", "Reset")}</button>
</div>

<style>
  .clock {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 4px;
    padding: 10px;
    color: var(--chrome-fg);
  }
  strong {
    font-size: 26px;
    font-variant-numeric: tabular-nums;
  }
  span {
    font-size: 14px;
    font-variant-numeric: tabular-nums;
    color: var(--chrome-muted);
  }
  button {
    padding: 2px 8px;
    font-size: 12px;
    color: inherit;
    background: none;
    border: 1px solid var(--chrome-line);
    border-radius: 4px;
    cursor: pointer;
  }
  button:focus-visible {
    outline: 2px solid var(--accent-ink);
  }
</style>
