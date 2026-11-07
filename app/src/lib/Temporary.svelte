<script lang="ts">
  // A folder the system empties: said, and nothing moved.
  let {
    tr,
    shown,
    label,
    act,
    dismiss,
  }: { tr: boolean; shown: string; label?: string; act?: () => void; dismiss?: () => void } = $props();
</script>

<div class="temporary" role="status">
  <span class="text">
    {tr ? "Bu klasör geçici: bilgisayar yeniden başlayınca silinebilir." : "This folder is temporary: it may be erased when the computer restarts."}
    <span class="where" title={shown}>{shown}</span>
  </span>
  {#if label && act}<button onclick={act}>{label}</button>{/if}
  {#if dismiss}<button title={tr ? "Kapat" : "Close"} onclick={dismiss}>✕</button>{/if}
</div>

<style>
  .temporary {
    display: flex;
    gap: 8px;
    align-items: center;
    min-width: 0;
    padding: 6px 10px;
    border-left: 3px solid var(--accent-ink);
    background: var(--chrome-2);
    color: var(--chrome-fg);
    font-size: 12px;
  }
  .text {
    flex: 1;
    min-width: 0;
  }
  .where {
    display: block;
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
    color: var(--chrome-muted);
  }
  button {
    flex: none;
    padding: 3px 8px;
    border: 1px solid var(--accent-ink);
    border-radius: 4px;
    background: none;
    color: var(--accent-ink);
    cursor: pointer;
  }
</style>
