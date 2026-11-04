<script lang="ts">
  // The board: each act a column of its scene cards. A card dragged to a
  // place moves its scene in the text (structure::relocate on the Rust
  // side); a dragged act takes its scenes along.
  import type { Card } from "./api";

  let {
    tr,
    cards,
    move,
    open,
    close,
    summaries = {},
  }: {
    tr: boolean;
    cards: Card[];
    /// Scene summaries by where their card starts.
    summaries?: Record<number, string>;
    move: (card: number, to: number) => void;
    open: (at: number) => void;
    close: () => void;
  } = $props();

  let dragging = $state<number | null>(null);
  let over = $state<number | null>(null);

  /// The acts with their scenes; scenes before the first act go first.
  const columns = $derived.by(() => {
    const cols: { act: number | null; scenes: number[] }[] = [];
    cards.forEach((c, i) => {
      if (c.act) cols.push({ act: i, scenes: [] });
      else {
        if (!cols.length) cols.push({ act: null, scenes: [] });
        cols[cols.length - 1].scenes.push(i);
      }
    });
    return cols;
  });

  /// Where a card can go: a place from its `places`.
  const can = (to: number) => dragging !== null && cards[dragging].places.includes(to) && to !== dragging;

  /// The place after a column's last scene: before the next act's card, or
  /// after everything.
  function endOf(ci: number) {
    const next = columns[ci + 1];
    return next?.act ?? cards.length;
  }

  /// Onto an act's card: a scene goes to the act's start, an act before it.
  function headTarget(ci: number) {
    const col = columns[ci];
    if (dragging !== null && cards[dragging].act) return col.act!;
    return col.scenes[0] ?? endOf(ci);
  }

  function drop(to: number) {
    if (dragging !== null && can(to)) move(dragging, to);
    dragging = null;
    over = null;
  }

  const zone = (to: number) => ({
    ondragover: (e: DragEvent) => {
      if (can(to)) {
        e.preventDefault();
        over = to;
      }
    },
    ondragleave: () => {
      if (over === to) over = null;
    },
    ondrop: (e: DragEvent) => {
      e.preventDefault();
      drop(to);
    },
  });
</script>

<div class="board">
  <div class="head">
    <strong>{tr ? "Pano" : "Board"}</strong>
    <span class="muted">
      {tr
        ? "Bir sahneyi sürükleyip başka yere bırak; perdeyi sürüklersen sahneleriyle gider. Çift tık: sahneye git."
        : "Drag a scene to another place; an act takes its scenes along. Double-click: go to the scene."}
    </span>
    <span class="grow"></span>
    <button onclick={close}>{tr ? "Sayfaya dön" : "Back to the page"} ✕</button>
  </div>
  <div class="cols">
    {#each columns as col, ci}
      <section class="col" class:target={col.act !== null && over === headTarget(ci)}>
        {#if col.act !== null}
          {@const a = cards[col.act]}
          <div
            class="act"
            class:hidden={a.hidden}
            draggable="true"
            role="listitem"
            ondragstart={() => (dragging = col.act)}
            ondragend={() => ((dragging = null), (over = null))}
            ondblclick={() => open(a.at)}
            {...zone(headTarget(ci))}
          >
            <div class="label">{a.label}</div>
            {#if a.title}<div class="title">{a.title}</div>{/if}
          </div>
        {:else}
          <div class="act none">{tr ? "Perdesiz" : "No act"}</div>
        {/if}
        {#each col.scenes as i}
          {@const c = cards[i]}
          <div class="slot" class:on={over === i} {...zone(i)}></div>
          <div
            class="card"
            class:hidden={c.hidden}
            class:lifted={dragging === i}
            draggable="true"
            role="listitem"
            ondragstart={(e) => {
              dragging = i;
              e.dataTransfer?.setData("text/plain", String(i));
            }}
            ondragend={() => ((dragging = null), (over = null))}
            ondblclick={() => open(c.at)}
            {...zone(i)}
          >
            <div class="label">{c.label}</div>
            <div class="title">{c.title || "—"}</div>
            {#if summaries[c.at]}<div class="summary">{summaries[c.at]}</div>{/if}
            <div class="lines">{c.lines} {tr ? "replik" : "lines"}</div>
          </div>
        {/each}
        <div class="slot end" class:on={over === endOf(ci)} {...zone(endOf(ci))}></div>
      </section>
    {/each}
  </div>
</div>

<style>
  .summary {
    margin: 4px 0 2px;
    font-family: var(--serif);
    font-style: italic;
    font-size: 12.5px;
    line-height: 1.35;
    color: var(--chrome-muted);
  }
  .board {
    display: flex;
    flex-direction: column;
    height: 100%;
    background: var(--desk);
  }
  .head {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 8px 14px;
    background: var(--chrome);
    border-bottom: 1px solid var(--chrome-line);
  }
  .head button {
    border: 1px solid var(--chrome-line);
    background: var(--chrome);
    border-radius: 4px;
    padding: 3px 10px;
    cursor: pointer;
  }
  .muted {
    color: var(--chrome-muted);
    font-size: 12px;
  }
  .grow {
    flex: 1;
  }
  .cols {
    flex: 1;
    display: flex;
    gap: 16px;
    padding: 18px;
    overflow: auto;
    align-items: flex-start;
  }
  .col {
    width: 230px;
    flex: none;
    background: color-mix(in srgb, var(--chrome) 70%, transparent);
    border-radius: 8px;
    padding: 10px;
    border: 2px solid transparent;
  }
  .col.target {
    border-color: var(--claret);
  }
  .act {
    padding: 8px 10px;
    border-radius: 6px;
    background: var(--claret);
    color: #f6efe4;
    cursor: grab;
    margin-bottom: 4px;
  }
  .act.none {
    background: var(--chrome-2);
    color: var(--chrome-muted);
    cursor: default;
  }
  .act .label {
    font-weight: 700;
    letter-spacing: 0.06em;
  }
  .act .title {
    font-size: 12px;
    opacity: 0.9;
  }
  .card {
    background: var(--page);
    color: var(--fg);
    border-radius: 6px;
    padding: 8px 10px;
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.18);
    cursor: grab;
    font-family: var(--serif);
  }
  .card.lifted {
    opacity: 0.4;
  }
  .card.hidden,
  .act.hidden {
    opacity: 0.55;
    font-style: italic;
  }
  .card .label {
    font-family: var(--sans);
    font-size: 11px;
    font-weight: 700;
    color: var(--heading);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }
  .card .title {
    font-size: 15px;
    line-height: 1.25;
    margin: 2px 0 4px;
  }
  .card .lines {
    font-family: var(--sans);
    font-size: 11px;
    color: var(--muted);
  }
  .slot {
    height: 8px;
    border-radius: 4px;
    margin: 2px 0;
  }
  .slot.end {
    height: 18px;
  }
  .slot.on {
    background: var(--claret);
    height: 18px;
  }
</style>
