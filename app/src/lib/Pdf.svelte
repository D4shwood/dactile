<script lang="ts">
  // The built PDF beside the page, drawn by pdf.js: the bars, the keys and
  // the mouse. The pages themselves are in pdf/sheets.ts.
  import { onDestroy, onMount, untrack } from "svelte";
  import * as pdfjs from "pdfjs-dist";
  import worker from "pdfjs-dist/build/pdf.worker.min.mjs?url";
  import { api, type BuildResult, type Viewer } from "./api";
  import BottomBar from "./pdf/BottomBar.svelte";
  import Icon from "./pdf/Icon.svelte";
  import { centimetres, clampZoom, stepOf, stepZoom, Trail, type Mode } from "./pdf/layout";
  import { Sheets, type Fit, type Place } from "./pdf/sheets";
  import TopBar from "./pdf/TopBar.svelte";

  pdfjs.GlobalWorkerOptions.workerSrc = worker;

  let {
    tr,
    result,
    building,
    close,
    goto,
    say,
  }: {
    tr: boolean;
    result: BuildResult | null;
    building: boolean;
    close: () => void;
    goto: (line: number) => void;
    say: (text: string, warn?: boolean) => void;
  } = $props();

  const l = (t: string, e: string) => (tr ? t : e);

  let root: HTMLElement | undefined = $state();
  let host: HTMLDivElement | undefined = $state();
  let box: HTMLInputElement | undefined = $state();
  let sheets: Sheets | undefined = $state();
  let o = $state<Viewer | null>(null);
  let mode = $state<Mode>("magnify");
  let page = $state(1);
  let pages = $state(0);
  let zoom = $state(100);
  let wide = $state(false);
  let turn = $state(0);
  let menu = $state(false);
  let failure = $state("");
  // A rectangle being dragged out, in the window's coordinates.
  let band = $state<{ x: number; y: number; w: number; h: number } | null>(null);

  const trail = new Trail<Place>();
  let canBack = $state(false);
  let canForward = $state(false);
  function trailed() {
    canBack = trail.past.length > 0;
    canForward = trail.ahead.length > 0;
  }

  // ------------------------------------------------------- the document

  // Which load is the latest; an older one stops where it stands.
  let drawn = 0;
  // Lets go of the document on the screen.
  let release: (() => void) | null = null;

  /// A document lives as long as its pages are on the screen. Each one is
  /// closed in `finally`, and its worker with it.
  async function show(path: string, n: number) {
    const bytes = new Uint8Array(await api.pdf(path));
    if (n !== drawn) return;
    const loading = pdfjs.getDocument({ data: bytes });
    try {
      const doc = await loading.promise;
      if (n !== drawn || !sheets) return;
      failure = "";
      await sheets.open(doc);
      // A newer build came meanwhile: this one is about to close.
      if (n !== drawn) return sheets.forget(doc);
      pages = doc.numPages;
      const ended = new Promise<void>((done) => {
        release?.();
        release = done;
      });
      await ended;
    } finally {
      await loading.destroy();
    }
  }

  $effect(() => {
    if (result?.kind === "built" && sheets && o) {
      const n = ++drawn;
      const path = result.pdf;
      untrack(() => {
        show(path, n).catch((e) => {
          if (n === drawn) failure = String(e);
        });
      });
    }
  });

  onMount(async () => {
    const v = (await api.settings()).viewer;
    if (!host) return;
    o = v;
    sheets = new Sheets(host, {
      page: (n) => (page = n),
      zoom: (z) => (zoom = z),
      error: (text) => (failure = text),
    });
    apply();
    sheets.fit(v.fit);
    if (!v.fit) sheets.setZoom(clampZoom(v.zoom));
  });

  // A drawing under way stops, and the document closes.
  onDestroy(() => {
    drawn++;
    sheets?.destroy();
    release?.();
    release = null;
    clearTimeout(saving);
  });

  // ------------------------------------------------------- the choices

  let saving = 0;
  /// The choices kept, a moment after the last change.
  function keep() {
    clearTimeout(saving);
    saving = window.setTimeout(() => {
      if (o) api.set("viewer", $state.snapshot(o)).catch((e) => say(String(e), true));
    }, 400);
  }

  function apply() {
    if (!o || !sheets) return;
    sheets.set({ columns: o.columns, rows: o.rows, offset: o.offset, single: o.single_step }, o.continuous, turn);
  }

  /// A choice changed: on the pages, and in the settings.
  function choose(patch: Partial<Viewer>) {
    if (!o) return;
    Object.assign(o, patch);
    apply();
    keep();
  }

  function fit(f: Fit) {
    if (!o) return;
    o.fit = f;
    sheets?.fit(f);
    keep();
  }

  function setZoom(z: number, x?: number, y?: number) {
    if (!o) return;
    o.fit = "";
    o.zoom = clampZoom(z);
    sheets?.setZoom(o.zoom, x, y);
    keep();
  }

  // ------------------------------------------------------- moving about

  /// A jump: the place left behind can be gone back to.
  function jump(act: () => void) {
    if (!sheets) return;
    trail.jump(sheets.place());
    act();
    trailed();
  }

  function go(to: Place) {
    jump(() => sheets?.restore(to));
  }

  function back() {
    const to = sheets && trail.back(sheets.place());
    if (to) sheets?.restore(to);
    trailed();
  }

  function forward() {
    const to = sheets && trail.forward(sheets.place());
    if (to) sheets?.restore(to);
    trailed();
  }

  const step = () => (o ? stepOf({ columns: o.columns, rows: o.rows, offset: o.offset, single: o.single_step }) : 1);

  function goPage(text: string) {
    const n = parseInt(text.trim(), 10);
    if (n >= 1 && n <= pages) jump(() => sheets?.go(n));
    else say(l(`Sayfa 1 ile ${pages} arasında olmalı.`, `The page must be from 1 to ${pages}.`), true);
  }

  function act(what: string) {
    const path = result?.kind === "built" ? result.pdf : null;
    switch (what) {
      case "external":
        if (path) api.launch(path).catch((e) => say(String(e), true));
        break;
      case "reveal":
        if (path) api.reveal(path).catch((e) => say(String(e), true));
        break;
      case "back":
        back();
        break;
      case "forward":
        forward();
        break;
      case "first":
        jump(() => sheets?.go(1));
        break;
      case "previous":
        sheets?.go(page - step());
        break;
      case "next":
        sheets?.go(page + step());
        break;
      case "last":
        jump(() => sheets?.go(pages));
        break;
      case "actual":
        setZoom(100);
        break;
      case "fit-width":
        fit("width");
        break;
      case "fit-text":
        fit("text");
        break;
      case "fit-window":
        fit("window");
        break;
      case "hide":
        if (o) choose({ toolbar: !o.toolbar });
        break;
      case "wide":
        wide = !wide;
        break;
      case "close":
        close();
        break;
    }
  }

  // ------------------------------------------------------- keys

  /// Keys act here only while the focus is in the viewer; those not taken
  /// go on to the writer.
  function onkey(e: KeyboardEvent) {
    const mod = e.ctrlKey || e.metaKey;
    const typing = (e.target as HTMLElement).matches?.("input, select, textarea");
    const take = (f: () => void) => {
      e.preventDefault();
      e.stopPropagation();
      f();
    };
    if (e.altKey && !mod) {
      if (e.key === "ArrowLeft") return take(back);
      if (e.key === "ArrowRight") return take(forward);
      return;
    }
    if (mod && !e.shiftKey) {
      if (e.key === "=" || e.key === "+") return take(() => setZoom(stepZoom(zoom, true)));
      if (e.key === "-") return take(() => setZoom(stepZoom(zoom, false)));
      if (e.key === "1") return take(() => setZoom(100));
      if (e.key === "2") return take(() => fit("width"));
      if (e.key === "3") return take(() => fit("window"));
      if (e.key === "4") return take(() => fit("text"));
      if (e.key === "j" || e.key === "J") return take(gotoBox);
      if (e.key === "Home") return take(() => act("first"));
      if (e.key === "End") return take(() => act("last"));
      return;
    }
    if (e.key === "Escape") return take(escape);
    if (typing || mod) return;
    if (e.key === "Home") return take(() => act("first"));
    if (e.key === "End") return take(() => act("last"));
    if (e.key === "ArrowLeft") return take(() => act("previous"));
    if (e.key === "ArrowRight") return take(() => act("next"));
    if (!o?.continuous && e.key === "PageDown") return take(() => act("next"));
    if (!o?.continuous && e.key === "PageUp") return take(() => act("previous"));
  }

  function gotoBox() {
    if (o && !o.toolbar) choose({ toolbar: true });
    requestAnimationFrame(() => box?.focus());
  }

  /// Esc closes one thing at a time, the last opened first.
  function escape() {
    if (menu) menu = false;
    else if (band) band = null;
    else if (wide) wide = false;
    else close();
  }

  // ------------------------------------------------------- the mouse

  function onwheel(e: WheelEvent) {
    if (!e.ctrlKey) return;
    e.preventDefault();
    const r = host!.getBoundingClientRect();
    setZoom(zoom * (e.deltaY < 0 ? 1.1 : 1 / 1.1), e.clientX - r.left, e.clientY - r.top);
  }

  /// The mouse's back and forward buttons.
  function onmouseup(e: MouseEvent) {
    if (e.button === 3) back();
    else if (e.button === 4) forward();
    else return;
    e.preventDefault();
  }

  /// Follows one drag to its end.
  function drag(e: PointerEvent, move: (m: PointerEvent) => void, up: (m: PointerEvent) => void) {
    const el = e.currentTarget as HTMLElement;
    el.setPointerCapture(e.pointerId);
    const end = (m: PointerEvent) => {
      el.removeEventListener("pointermove", move);
      el.removeEventListener("pointerup", end);
      el.removeEventListener("pointercancel", end);
      up(m);
    };
    el.addEventListener("pointermove", move);
    el.addEventListener("pointerup", end);
    el.addEventListener("pointercancel", end);
  }

  function ondown(e: PointerEvent) {
    if (e.button !== 0 || !sheets || !host) return;
    if (e.target === host && e.offsetX > host.clientWidth) return; // Its scroll bar.
    const mod = e.ctrlKey || e.metaKey;
    if (mod && e.shiftKey) {
      e.preventDefault();
      const s = sheets.at(e.clientX, e.clientY);
      if (s) copyPlace(s.x, s.y);
      return;
    }
    if (mode === "scroll") {
      e.preventDefault();
      const [x, y, left, top] = [e.clientX, e.clientY, host.scrollLeft, host.scrollTop];
      drag(
        e,
        (m) => {
          host!.scrollLeft = left - (m.clientX - x);
          host!.scrollTop = top - (m.clientY - y);
        },
        () => {},
      );
      return;
    }
    if (mode !== "magnify") return;
    e.preventDefault();
    if (e.altKey) return setZoom(stepZoom(zoom, false), ...inside(e));
    if (!e.shiftKey) return;
    const [x, y] = [e.clientX, e.clientY];
    band = { x, y, w: 0, h: 0 };
    drag(
      e,
      (m) => {
        if (band) band = { x: Math.min(x, m.clientX), y: Math.min(y, m.clientY), w: Math.abs(m.clientX - x), h: Math.abs(m.clientY - y) };
      },
      (m) => {
        const b = band;
        band = null;
        if (!b) return;
        if (b.w > 8 && b.h > 8) {
          if (o) o.fit = "";
          sheets?.zoomTo(b.x, b.y, b.x + b.w, b.y + b.h);
          if (o && sheets) o.zoom = sheets.zoom;
          keep();
        } else setZoom(stepZoom(zoom, true), ...inside(m));
      },
    );
  }

  /// A point of the window, in the view.
  function inside(e: MouseEvent): [number, number] {
    const r = host!.getBoundingClientRect();
    return [e.clientX - r.left, e.clientY - r.top];
  }

  function copyPlace(x: number, y: number) {
    const text = centimetres(x, y);
    navigator.clipboard
      ?.writeText(text)
      .then(() => say(l(`Konum kopyalandı: ${text} cm`, `Position copied: ${text} cm`)))
      .catch((e) => say(String(e), true));
  }

  const none = $derived(result?.kind !== "built" && pages === 0);
  const grids: [number, number][] = [
    [1, 1],
    [2, 1],
    [1, 2],
    [2, 2],
    [3, 1],
    [3, 2],
    [3, 3],
  ];
</script>

{#snippet more()}
  <span class="menu">
    <button title={l("Görünüm seçenekleri", "View options")} aria-label={l("Görünüm seçenekleri", "View options")} aria-expanded={menu} class:on={menu} onclick={() => (menu = !menu)}><Icon name="more" /></button>
    {#if menu && o}
      <div class="drop">
        <label><input type="checkbox" checked={o.continuous} onchange={(e) => choose({ continuous: e.currentTarget.checked })} /> {l("Sürekli", "Continuous")}</label>
        <label><input type="checkbox" checked={o.single_step} onchange={(e) => choose({ single_step: e.currentTarget.checked })} /> {l("Tek sayfa adımı", "Single page step")}</label>
        <div class="row">
          <span>{l("Izgara", "Grid")}</span>
          {#each grids as [c, r] (`${c}x${r}`)}
            <button class="word" class:on={o.columns === c && o.rows === r} title={`${c}×${r}`} onclick={() => choose({ columns: c, rows: r })}>{c}×{r}</button>
          {/each}
        </div>
        <div class="row">
          <span>{l("Özel", "Custom")}</span>
          <input type="number" min="1" max="9" value={o.columns} title={l("Sütun", "Columns")} aria-label={l("Sütun", "Columns")} onchange={(e) => choose({ columns: Math.min(9, Math.max(1, +e.currentTarget.value || 1)) })} />
          ×
          <input type="number" min="1" max="9" value={o.rows} title={l("Satır", "Rows")} aria-label={l("Satır", "Rows")} onchange={(e) => choose({ rows: Math.min(9, Math.max(1, +e.currentTarget.value || 1)) })} />
        </div>
        <div class="row">
          <span>{l("Başlangıç sütunu", "Start column")}</span>
          <input type="number" min="1" max={o.columns} value={o.offset + 1} title={l("İlk sayfanın sütunu", "The first page's column")} aria-label={l("Başlangıç sütunu", "Start column")} onchange={(e) => choose({ offset: Math.min(o!.columns, Math.max(1, +e.currentTarget.value || 1)) - 1 })} />
        </div>
      </div>
    {/if}
  </span>
{/snippet}

<!-- svelte-ignore a11y_no_noninteractive_tabindex, a11y_no_noninteractive_element_interactions -->
<aside class="pdf" class:wide bind:this={root} tabindex="-1" onkeydown={onkey} onmouseup={onmouseup}>
  {#if o?.toolbar}
    <TopBar {tr} bind:mode bind:box fit={o.fit} {page} {pages} label="" {canBack} {canForward} {wide} disabled={none} {act} go={goPage} {more} />
  {:else if o}
    <button class="unhide" title={l("Araç çubuğunu göster", "Show toolbar")} aria-label={l("Araç çubuğunu göster", "Show toolbar")} onclick={() => act("hide")}><Icon name="show" /></button>
  {/if}
  {#if result?.kind === "failed"}
    <div class="error">
      <strong>{tr ? "PDF çıkmadı" : "No PDF"}</strong>
      <p>{result.message}</p>
      {#if result.line}
        <button onclick={() => goto(result.line!)}>
          {tr ? `${result.line}. satıra git` : `Go to line ${result.line}`}
        </button>
      {/if}
    </div>
  {:else if failure}
    <div class="error" role="alert">
      <strong>{l("PDF gösterilemedi", "The PDF can't be shown")}</strong>
      <p>{failure}</p>
    </div>
  {:else if !result && !building}
    <p class="muted empty">{tr ? "F5 ile derle." : "Build with F5."}</p>
  {/if}
  {#if building}<p class="muted note">{tr ? "derleniyor…" : "building…"}</p>{/if}
  <!-- svelte-ignore a11y_no_noninteractive_tabindex, a11y_no_static_element_interactions -->
  <div class="pages {mode}" class:invert={o?.invert} class:gray={o?.gray} class:dim={building} bind:this={host} tabindex="0" onpointerdown={ondown} {onwheel}></div>
  <BottomBar {tr} left={!!o?.left} right={!!o?.right} {page} {pages} {zoom} disabled={none} {setZoom} step={(up) => setZoom(stepZoom(zoom, up))} />
  {#if band}
    <div class="band" style:left="{band.x}px" style:top="{band.y}px" style:width="{band.w}px" style:height="{band.h}px"></div>
  {/if}
</aside>

<style>
  .pdf {
    position: relative;
    display: flex;
    flex-direction: column;
    min-width: 0;
    min-height: 0;
    outline: none;
    background: var(--desk);
    border-left: 1px solid var(--chrome-line);
  }
  .pdf.wide {
    position: fixed;
    inset: 0;
    z-index: 40;
    border: 0;
  }
  .unhide {
    position: absolute;
    top: 4px;
    right: 18px;
    z-index: 3;
    display: inline-flex;
    padding: 3px;
    border: 1px solid var(--chrome-line);
    border-radius: 4px;
    background: var(--chrome);
    color: var(--chrome-fg);
    cursor: pointer;
  }
  .menu {
    position: relative;
  }
  .drop {
    position: absolute;
    top: 30px;
    right: 0;
    z-index: 6;
    display: flex;
    flex-direction: column;
    gap: 6px;
    width: max-content;
    max-width: 280px;
    padding: 8px 10px;
    font-size: 12px;
    color: var(--chrome-fg);
    background: var(--chrome);
    border: 1px solid var(--chrome-line);
    border-radius: 6px;
    box-shadow: 0 6px 18px rgba(0, 0, 0, 0.25);
  }
  .drop label,
  .row {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 5px;
  }
  .drop .word {
    width: auto;
    height: 22px;
    padding: 0 5px;
    font-size: 11px;
    border-color: var(--chrome-line);
  }
  .drop input[type="number"] {
    width: 3.2em;
    font: inherit;
    color: inherit;
    background: var(--chrome-2);
    border: 1px solid var(--chrome-line);
    border-radius: 3px;
    caret-color: var(--claret);
  }
  .muted {
    color: var(--chrome-muted);
    font-size: 12px;
  }
  .empty {
    padding: 16px;
    margin: 0;
  }
  .note {
    position: absolute;
    bottom: 34px;
    left: 12px;
    z-index: 3;
    margin: 0;
    padding: 2px 8px;
    background: var(--chrome);
    border-radius: 4px;
  }
  .pages {
    flex: 1;
    min-height: 0;
    overflow: auto;
    position: relative;
    outline: none;
  }
  .pages.dim {
    opacity: 0.5;
  }
  .pages.magnify {
    cursor:
      url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='24' height='24'%3E%3Ccircle cx='10' cy='10' r='7' fill='none' stroke='%23000' stroke-width='3'/%3E%3Ccircle cx='10' cy='10' r='7' fill='none' stroke='%23fff' stroke-width='1.5'/%3E%3Cpath d='M15 15l6 6' stroke='%23000' stroke-width='3'/%3E%3C/svg%3E")
        10 10,
      zoom-in;
  }
  .pages.scroll {
    cursor: grab;
  }
  .pages.scroll:active {
    cursor: grabbing;
  }
  .pages :global(.grid) {
    display: grid;
    gap: 14px;
    width: max-content;
    margin: 0 auto;
    padding: 16px;
    position: relative;
  }
  .pages :global(.sheet) {
    position: relative;
    overflow: hidden;
    background: white;
    box-shadow: 0 2px 10px rgba(0, 0, 0, 0.25);
  }
  .pages :global(.sheet[hidden]) {
    display: none;
  }
  .pages :global(.sheet canvas) {
    display: block;
    width: 100%;
    height: 100%;
  }
  .pages.invert :global(canvas) {
    filter: invert(1) hue-rotate(180deg);
  }
  .pages.gray :global(canvas) {
    filter: grayscale(1);
  }
  .pages.invert.gray :global(canvas) {
    filter: invert(1) grayscale(1);
  }
  /* pdf.js's text layer: invisible text over the canvas, to select. */
  .pages :global(.textLayer) {
    position: absolute;
    inset: 0;
    overflow: clip;
    line-height: 1;
    text-align: initial;
    transform-origin: 0 0;
    text-size-adjust: none;
    pointer-events: none;
    user-select: none;
    -webkit-user-select: none;
  }
  .pages.select :global(.textLayer) {
    pointer-events: auto;
    user-select: text;
    -webkit-user-select: text;
  }
  .pages :global(.textLayer span),
  .pages :global(.textLayer br) {
    position: absolute;
    color: transparent;
    white-space: pre;
    cursor: text;
    transform-origin: 0% 0%;
  }
  .pages :global(.textLayer span.markedContent) {
    top: 0;
    height: 0;
  }
  .pages :global(.textLayer ::selection) {
    color: transparent;
    background: rgba(176, 68, 74, 0.3);
  }
  .pages :global(.textLayer .endOfContent) {
    position: absolute;
    inset: 100% 0 0;
    display: block;
    z-index: 0;
    cursor: default;
    user-select: none;
    -webkit-user-select: none;
  }
  .pages :global(.textLayer.selecting .endOfContent) {
    top: 0;
  }
  .pages :global(.textLayer[data-main-rotation="90"]) {
    transform: rotate(90deg) translateY(-100%);
  }
  .pages :global(.textLayer[data-main-rotation="180"]) {
    transform: rotate(180deg) translate(-100%, -100%);
  }
  .pages :global(.textLayer[data-main-rotation="270"]) {
    transform: rotate(270deg) translateX(-100%);
  }
  .band {
    position: fixed;
    z-index: 5;
    pointer-events: none;
    border: 1px solid var(--claret);
    background: color-mix(in srgb, var(--claret) 15%, transparent);
  }
  .error {
    margin: 12px;
    padding: 10px 12px;
    border-left: 3px solid var(--claret);
    background: var(--chrome);
    border-radius: 4px;
  }
  .error p {
    white-space: pre-wrap;
    font: 12px/1.4 var(--mono);
    margin: 6px 0;
  }
  .error button {
    border: 1px solid var(--claret);
    color: var(--claret);
    background: none;
    border-radius: 4px;
    padding: 3px 10px;
    cursor: pointer;
  }
</style>
