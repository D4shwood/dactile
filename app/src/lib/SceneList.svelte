<script lang="ts">
  // The play's headings down the left of the writing, as in the terminal
  // app's margin: a click goes there, the cursor's one is marked. It folds
  // away to a strip, an act folds its scenes, its edge is dragged wider, and
  // a scene or act is dragged to another place, as on the board.
  import type { Card, Head } from "./api";

  let {
    tr,
    heads,
    cards,
    at,
    open,
    width,
    locked,
    go,
    move,
    toggle,
    resized,
  }: {
    tr: boolean;
    heads: Head[];
    /// The board's cards, for where a heading may go.
    cards: Card[];
    at: number;
    open: boolean;
    /// In pixels, dragged by hand; 0 is its own.
    width: number;
    locked: boolean;
    go: (from: number) => void;
    move: (card: number, to: number) => void;
    toggle: () => void;
    resized: (w: number) => void;
  } = $props();

  const l = (t: string, e: string) => (tr ? t : e);
  const OWN = 220;
  const LEAST = 140;
  const MOST = 440;

  const here = $derived(heads.filter((h) => h.from <= at).length - 1);
  let list: HTMLElement | undefined = $state();
  $effect(() => {
    void here;
    list?.querySelector(".on")?.scrollIntoView({ block: "nearest" });
  });

  // Folded acts, by their place among the acts: it stays while typing moves
  // their text.
  let folded = $state<number[]>([]);
  const actOf = $derived(
    heads.map((_, i) => {
      let n = -1;
      for (let j = 0; j <= i; j++) if (heads[j].depth === 0) n++;
      return n;
    }),
  );
  const shown = (i: number) => heads[i].depth === 0 || !folded.includes(actOf[i]);
  function fold(i: number) {
    const n = actOf[i];
    folded = folded.includes(n) ? folded.filter((f) => f !== n) : [...folded, n];
  }

  // ---------------------------------------------------------- dragging

  /// The card a heading is: the last one starting at or before it.
  function cardOf(h: Head): number {
    let c = -1;
    cards.forEach((card, j) => {
      if (card.at <= h.from) c = j;
    });
    return c;
  }
  let dragging = $state<number | null>(null);
  /// Where it would go: before this heading, or past the last one (-1).
  let over = $state<number | null>(null);
  const placeOf = (i: number) => (i < 0 ? cards.length : cardOf(heads[i]));
  const can = (to: number) => dragging !== null && dragging >= 0 && to !== dragging && !!cards[dragging]?.places.includes(to);

  const zone = (i: number) => ({
    ondragover: (e: DragEvent) => {
      if (!can(placeOf(i))) return;
      e.preventDefault();
      over = i;
    },
    ondragleave: () => {
      if (over === i) over = null;
    },
    ondrop: (e: DragEvent) => {
      e.preventDefault();
      const to = placeOf(i);
      if (dragging !== null && can(to)) move(dragging, to);
      dragging = over = null;
    },
  });

  // ------------------------------------------------------------ width

  let dragged = $state(0);
  const shownWidth = $derived(dragged || width || OWN);
  function dragEdge(e: PointerEvent) {
    const edge = e.currentTarget as HTMLElement;
    const startX = e.clientX;
    const from = shownWidth;
    edge.setPointerCapture(e.pointerId);
    const moveTo = (m: PointerEvent) => {
      dragged = Math.round(Math.min(MOST, Math.max(LEAST, from + m.clientX - startX)));
    };
    const up = () => {
      edge.removeEventListener("pointermove", moveTo);
      edge.removeEventListener("pointerup", up);
      if (dragged) resized(dragged);
      dragged = 0;
    };
    edge.addEventListener("pointermove", moveTo);
    edge.addEventListener("pointerup", up);
    e.preventDefault();
  }
</script>

{#if open}
  <nav class="scene-list" aria-label={l("Sahneler", "Scenes")} style:width="{shownWidth}px" style:--list-w="{shownWidth}px">
    <div class="bar">
      <span>{l("Sahneler", "Scenes")}</span>
      <button class="hide" title={l("Listeyi gizle", "Hide the list")} onclick={toggle}>‹</button>
    </div>
    <div class="rows" bind:this={list}>
      {#each heads as h, i (h.from)}
        {#if shown(i)}
          <div
            class="row"
            class:top={h.depth === 0}
            class:on={i === here}
            class:hidden={h.hidden}
            class:over={over === i}
            class:dragging={dragging !== null && dragging === cardOf(h)}
            draggable={!locked && cards.length > 0}
            ondragstart={(e) => {
              dragging = cardOf(h);
              e.dataTransfer?.setData("text/plain", h.label);
              if (e.dataTransfer) e.dataTransfer.effectAllowed = "move";
            }}
            ondragend={() => (dragging = over = null)}
            role="listitem"
            {...zone(i)}
          >
            {#if h.depth === 0}
              <button class="fold" title={folded.includes(actOf[i]) ? l("Sahneleri göster", "Show its scenes") : l("Sahneleri katla", "Fold its scenes")} onclick={() => fold(i)}>
                {folded.includes(actOf[i]) ? "▸" : "▾"}
              </button>
            {/if}
            <button class="go" title={h.label} onclick={() => go(h.from)}>{h.label}</button>
          </div>
        {/if}
      {/each}
      <div class="end" class:over={over === -1} role="listitem" {...zone(-1)}></div>
    </div>
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div
      class="edge"
      title={l("Sürükle; çift tık: varsayılan", "Drag; double click: its own width")}
      onpointerdown={dragEdge}
      ondblclick={() => resized(0)}
    ></div>
  </nav>
{:else}
  <button class="rail" title={l("Sahneleri göster", "Show the scenes")} aria-label={l("Sahneleri göster", "Show the scenes")} onclick={toggle}>›</button>
{/if}

<style>
  .scene-list {
    position: absolute;
    inset: 0 auto 0 0;
    /* Over the page's edge: the handle reaches past it. */
    z-index: 2;
    display: flex;
    flex-direction: column;
    background: var(--desk);
    border-right: 1px solid var(--chrome-line);
    font: 12.5px var(--sans, system-ui);
  }
  .bar {
    display: flex;
    align-items: center;
    padding: 6px 4px 2px 14px;
    color: var(--chrome-muted);
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.05em;
    text-transform: uppercase;
  }
  .bar span {
    flex: 1;
  }
  .hide,
  .fold,
  .rail {
    padding: 0;
    color: var(--chrome-muted);
    background: none;
    border: 0;
    cursor: pointer;
  }
  .hide {
    width: 24px;
    height: 22px;
    font-size: 16px;
    border-radius: 4px;
  }
  .hide:hover,
  .rail:hover {
    color: var(--accent-ink);
    background: var(--chrome-2);
  }
  .rows {
    flex: 1;
    min-height: 0;
    padding: 2px 6px 40px;
    overflow-y: auto;
  }
  .row {
    display: flex;
    align-items: center;
    border-top: 2px solid transparent;
    border-radius: 4px;
  }
  .row.over,
  .end.over {
    border-top-color: var(--accent-ink);
  }
  .row.dragging {
    opacity: 0.4;
  }
  .row:hover {
    background: var(--chrome-2);
  }
  .fold {
    flex: none;
    width: 16px;
    font-size: 10px;
  }
  .go {
    flex: 1;
    min-width: 0;
    padding: 3px 8px 3px 20px;
    overflow: hidden;
    color: var(--faint, var(--chrome-muted));
    font: inherit;
    text-align: left;
    text-overflow: ellipsis;
    white-space: nowrap;
    background: none;
    border: 0;
    cursor: pointer;
  }
  .row.top {
    margin-top: 6px;
  }
  .row.top .go {
    padding-left: 2px;
    color: var(--chrome-muted);
    font-weight: 600;
  }
  .row.hidden .go {
    font-style: italic;
  }
  .row.on .go {
    color: var(--accent-ink);
    font-weight: 600;
  }
  .row[draggable="true"] {
    cursor: grab;
  }
  .end {
    height: 24px;
    border-top: 2px solid transparent;
  }
  .edge {
    position: absolute;
    top: 0;
    right: -4px;
    bottom: 0;
    width: 7px;
    cursor: col-resize;
    touch-action: none;
  }
  .edge:hover {
    background: var(--chrome-line);
  }
  .rail {
    position: absolute;
    inset: 0 auto 0 0;
    width: 18px;
    font-size: 14px;
    background: var(--desk);
    border-right: 1px solid var(--chrome-line);
    border-radius: 0;
  }
</style>
