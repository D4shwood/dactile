<script lang="ts">
  // The viewer's bottom bar: the side panels, the page, the zoom.
  import Icon from "./Icon.svelte";
  import { MAX_ZOOM, MIN_ZOOM, STOPS } from "./layout";

  let {
    tr,
    hasLeft = false,
    left,
    right,
    page,
    pages,
    zoom,
    disabled,
    toggle,
    setZoom,
    step,
  }: {
    tr: boolean;
    hasLeft?: boolean;
    left: boolean;
    right: boolean;
    page: number;
    pages: number;
    zoom: number;
    disabled: boolean;
    toggle: (side: "left" | "right") => void;
    setZoom: (z: number) => void;
    step: (up: boolean) => void;
  } = $props();

  const l = (t: string, e: string) => (tr ? t : e);
  let custom = $state(false);
  // The slider runs on powers of two: 100 % sits in the middle of its range.
  const toSlider = (z: number) => Math.log2(z / 100);
  const stop = $derived(STOPS.find((s) => Math.abs(s - zoom) < 0.05));
  const shown = $derived(`${Math.round(zoom * 10) / 10}`);
</script>

<div class="bar">
  {#if hasLeft}
    <button title={l("Sol panel: küçük resimler, ana hat, bilgi", "Left panel: thumbnails, outline, info")} aria-label={l("Sol panel", "Left panel")} aria-pressed={left} class:on={left} onclick={() => toggle("left")}><Icon name="left" /></button>
  {/if}
  <button title={l("Sağ panel: arama, saat", "Right panel: search, clock")} aria-label={l("Sağ panel", "Right panel")} aria-pressed={right} class:on={right} onclick={() => toggle("right")}><Icon name="right" /></button>
  <span class="grow"></span>
  <span class="page">{l("Sayfa", "Page")} {page} / {pages}</span>
  {#if custom}
    <input
      class="custom"
      type="number"
      min={MIN_ZOOM}
      max={MAX_ZOOM}
      step="0.5"
      value={shown}
      title={l("Özel yakınlık, yüzde", "Custom zoom, percent")}
      aria-label={l("Özel yakınlık", "Custom zoom")}
      {disabled}
      onchange={(e) => {
        const z = parseFloat(e.currentTarget.value);
        if (z) setZoom(z);
        custom = false;
      }}
      onblur={() => (custom = false)}
    />
  {:else}
    <select
      title={l("Yakınlık", "Zoom")}
      aria-label={l("Yakınlık", "Zoom")}
      {disabled}
      value={stop ?? "now"}
      onchange={(e) => {
        const v = e.currentTarget.value;
        if (v === "custom") custom = true;
        else if (v !== "now") setZoom(parseFloat(v));
      }}
    >
      {#if stop === undefined}<option value="now">{shown} %</option>{/if}
      {#each STOPS as s (s)}<option value={s}>{s} %</option>{/each}
      <option value="custom">{l("Özel…", "Custom…")}</option>
    </select>
  {/if}
  <button title={l("Küçült (Ctrl+-)", "Zoom out (Ctrl+-)")} aria-label={l("Küçült", "Zoom out")} {disabled} onclick={() => step(false)}><Icon name="minus" /></button>
  <input
    class="slider"
    type="range"
    min={toSlider(MIN_ZOOM)}
    max={toSlider(MAX_ZOOM)}
    step="0.01"
    value={toSlider(zoom)}
    title={l("Yakınlık", "Zoom")}
    aria-label={l("Yakınlık", "Zoom")}
    {disabled}
    oninput={(e) => setZoom(100 * 2 ** parseFloat(e.currentTarget.value))}
  />
  <button title={l("Büyüt (Ctrl++)", "Zoom in (Ctrl++)")} aria-label={l("Büyüt", "Zoom in")} {disabled} onclick={() => step(true)}><Icon name="plus" /></button>
</div>

<style>
  .bar {
    display: flex;
    align-items: center;
    gap: 3px;
    padding: 2px 6px;
    background: var(--chrome);
    color: var(--chrome-fg);
    border-top: 1px solid var(--chrome-line);
    font-size: 12px;
  }
  .grow {
    flex: 1;
  }
  .page {
    margin-right: 6px;
    white-space: nowrap;
  }
  button {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 24px;
    height: 24px;
    padding: 0;
    border: 1px solid transparent;
    border-radius: 4px;
    background: none;
    color: inherit;
    cursor: pointer;
  }
  button:hover:not(:disabled),
  button.on {
    background: var(--chrome-2);
  }
  button.on {
    border-color: var(--chrome-line);
    color: var(--claret);
  }
  button:disabled {
    opacity: 0.35;
  }
  button:focus-visible,
  select:focus-visible,
  input:focus-visible {
    outline: 2px solid var(--claret);
    outline-offset: -1px;
  }
  select,
  .custom {
    height: 22px;
    font: inherit;
    color: inherit;
    background: var(--chrome-2);
    border: 1px solid var(--chrome-line);
    border-radius: 3px;
    caret-color: var(--claret);
  }
  .custom {
    width: 5em;
    box-sizing: border-box;
  }
  .slider {
    width: 110px;
    min-width: 40px;
    flex-shrink: 1;
    accent-color: var(--claret);
  }
</style>
