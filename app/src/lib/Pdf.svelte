<script lang="ts">
  // The built PDF beside the page, drawn by pdf.js.
  import { onDestroy } from "svelte";
  import * as pdfjs from "pdfjs-dist";
  import worker from "pdfjs-dist/build/pdf.worker.min.mjs?url";
  import { api, type BuildResult } from "./api";

  pdfjs.GlobalWorkerOptions.workerSrc = worker;

  let {
    tr,
    result,
    building,
    close,
    goto,
  }: {
    tr: boolean;
    result: BuildResult | null;
    building: boolean;
    close: () => void;
    goto: (line: number) => void;
  } = $props();

  let pages: HTMLDivElement | undefined = $state();
  let scale = $state(1);
  let drawn = 0;

  async function draw(path: string, n: number) {
    const bytes = new Uint8Array(await api.pdf(path));
    if (n !== drawn) return;
    const loading = pdfjs.getDocument({ data: bytes });
    // Each document has a worker of its own, there until it is closed.
    try {
      const doc = await loading.promise;
      if (n !== drawn || !pages) return;
      const keep = pages.scrollTop;
      const width = pages.clientWidth - 32;
      const out: HTMLCanvasElement[] = [];
      for (let i = 1; i <= doc.numPages; i++) {
        const page = await doc.getPage(i);
        const base = page.getViewport({ scale: 1 });
        const vp = page.getViewport({ scale: (width / base.width) * scale * devicePixelRatio });
        const c = document.createElement("canvas");
        c.width = vp.width;
        c.height = vp.height;
        c.style.width = `${vp.width / devicePixelRatio}px`;
        await page.render({ canvasContext: c.getContext("2d")!, viewport: vp }).promise;
        if (n !== drawn) return;
        out.push(c);
      }
      pages.replaceChildren(...out);
      pages.scrollTop = keep;
    } finally {
      await loading.destroy();
    }
  }

  $effect(() => {
    if (result?.kind === "built") {
      const n = ++drawn;
      void scale;
      draw(result.pdf, n);
    }
  });

  // A drawing under way stops at its next page.
  onDestroy(() => drawn++);
</script>

<aside class="pdf">
  <div class="head">
    <strong>PDF</strong>
    {#if building}
      <span class="muted">{tr ? "derleniyor…" : "building…"}</span>
    {:else if result?.kind === "built"}
      <span class="muted">
        {result.pages ?? "?"}
        {tr ? "sayfa" : "pages"} · {result.seconds.toFixed(1)} sn
      </span>
    {/if}
    <span class="grow"></span>
    <button title="−" onclick={() => (scale = Math.max(0.5, scale - 0.1))}>−</button>
    <button title="+" onclick={() => (scale = Math.min(2.5, scale + 0.1))}>+</button>
    <button title={tr ? "Kapat" : "Close"} onclick={close}>✕</button>
  </div>
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
  {:else if !result && !building}
    <p class="muted empty">{tr ? "F5 ile derle." : "Build with F5."}</p>
  {/if}
  <div class="pages" bind:this={pages} class:dim={building}></div>
</aside>

<style>
  .pdf {
    display: flex;
    flex-direction: column;
    min-width: 0;
    background: var(--desk);
    border-left: 1px solid var(--chrome-line);
  }
  .head {
    display: flex;
    gap: 8px;
    align-items: center;
    padding: 6px 10px;
    background: var(--chrome);
    border-bottom: 1px solid var(--chrome-line);
  }
  .head button {
    border: 1px solid var(--chrome-line);
    background: var(--chrome);
    border-radius: 4px;
    width: 26px;
    height: 24px;
    cursor: pointer;
  }
  .grow {
    flex: 1;
  }
  .muted {
    color: var(--chrome-muted);
    font-size: 12px;
  }
  .empty {
    padding: 16px;
  }
  .pages {
    flex: 1;
    overflow: auto;
    padding: 16px;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 14px;
  }
  .pages.dim {
    opacity: 0.5;
  }
  .pages :global(canvas) {
    box-shadow: 0 2px 10px rgba(0, 0, 0, 0.25);
    background: white;
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
