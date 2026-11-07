<script lang="ts">
  // The left panel: thumbnails, the outline, the document's info, its fonts.
  import { onDestroy } from "svelte";
  import { OPS, type PDFDocumentProxy } from "pdfjs-dist";
  import Outline, { type Entry } from "./Outline.svelte";

  let {
    tr,
    tab,
    doc,
    page,
    bytes,
    labels,
    choose,
    go,
    follow,
  }: {
    tr: boolean;
    tab: string;
    doc: PDFDocumentProxy | null;
    page: number;
    bytes: number;
    labels: string[] | null;
    choose: (tab: string) => void;
    go: (n: number) => void;
    /// An outline entry's destination, or web address.
    follow: (dest: unknown, url?: string) => void;
  } = $props();

  const l = (t: string, e: string) => (tr ? t : e);
  const TABS: [string, string, string][] = [
    ["thumbs", "Sayfalar", "Pages"],
    ["outline", "Ana hat", "Outline"],
    ["info", "Bilgi", "Info"],
    ["fonts", "Yazı tipleri", "Fonts"],
  ];
  const THUMB = 130;

  // ---- Thumbnails: an empty box for each page; those in sight are drawn.
  let list: HTMLDivElement | undefined = $state();
  let drawnThumbs = new Map<number, HTMLCanvasElement>();
  let watcher: IntersectionObserver | null = null;

  async function thumb(box: HTMLElement, n: number, d: PDFDocumentProxy) {
    if (drawnThumbs.has(n)) return;
    const canvas = document.createElement("canvas");
    drawnThumbs.set(n, canvas);
    try {
      const p = await d.getPage(n);
      const base = p.getViewport({ scale: 1 });
      const view = p.getViewport({ scale: THUMB / base.width });
      canvas.width = view.width;
      canvas.height = view.height;
      await p.render({ canvasContext: canvas.getContext("2d")!, viewport: view }).promise;
      if (drawnThumbs.get(n) === canvas) box.prepend(canvas);
    } catch {
      drawnThumbs.delete(n);
    }
  }

  function forget(n: number) {
    const c = drawnThumbs.get(n);
    if (!c) return;
    c.remove();
    c.width = 0;
    drawnThumbs.delete(n);
  }

  $effect(() => {
    const d = doc;
    if (tab !== "thumbs" || !d || !list) return;
    watcher = new IntersectionObserver(
      (seen) => {
        for (const s of seen) {
          const n = Number((s.target as HTMLElement).dataset.n);
          if (s.isIntersecting) void thumb(s.target as HTMLElement, n, d);
          else forget(n);
        }
      },
      { root: list, rootMargin: "300px" },
    );
    for (const el of list.querySelectorAll("[data-n]")) watcher.observe(el);
    return () => {
      watcher?.disconnect();
      for (const n of [...drawnThumbs.keys()]) forget(n);
    };
  });

  // The page in view stays in sight in the list.
  $effect(() => {
    list?.querySelector(`[data-n="${page}"]`)?.scrollIntoView({ block: "nearest" });
  });

  // ---- Outline and info, read when the document changes.
  let outline = $state.raw<Entry[] | null>(null);
  let info = $state.raw<[string, string][]>([]);

  /// PDF dates: D:20261106230000+03'00'.
  function when(s: unknown): string {
    if (typeof s !== "string") return "";
    const m = s.match(/^D:(\d{4})(\d{2})?(\d{2})?(\d{2})?(\d{2})?/);
    if (!m) return s;
    return `${m[1]}-${m[2] ?? "01"}-${m[3] ?? "01"} ${m[4] ?? "00"}:${m[5] ?? "00"}`;
  }

  $effect(() => {
    const d = doc;
    outline = null;
    info = [];
    if (!d) return;
    let live = true;
    void (async () => {
      try {
        const [o, m, first] = await Promise.all([d.getOutline(), d.getMetadata(), d.getPage(1)]);
        if (!live) return;
        outline = o as Entry[] | null;
        const i = m.info as Record<string, unknown>;
        const [w, h] = [first.view[2] - first.view[0], first.view[3] - first.view[1]];
        const cm = (p: number) => ((p / 72) * 2.54).toFixed(1);
        const rows: [string, unknown][] = [
          [l("Başlık", "Title"), i.Title],
          [l("Yazar", "Author"), i.Author],
          [l("Konu", "Subject"), i.Subject],
          [l("Anahtar sözcükler", "Keywords"), i.Keywords],
          [l("Oluşturan", "Creator"), i.Creator],
          [l("Üretici", "Producer"), i.Producer],
          [l("Oluşturma", "Created"), when(i.CreationDate)],
          [l("Değişiklik", "Modified"), when(i.ModDate)],
          [l("PDF sürümü", "PDF version"), i.PDFFormatVersion],
          [l("Sayfa", "Pages"), d.numPages],
          [l("Sayfa boyu", "Page size"), `${cm(w)} × ${cm(h)} cm`],
          [l("Dosya boyu", "File size"), `${(bytes / 1024).toFixed(0)} KB`],
        ];
        info = rows.filter(([, v]) => v !== undefined && v !== "").map(([k, v]) => [k, String(v)]);
      } catch {
        // The document closed meanwhile.
      }
    })();
    return () => {
      live = false;
    };
  });

  // ---- Fonts: pdf.js tells them page by page, once a page's drawing
  // orders are read. Only when asked: it goes through every page.
  let fonts = $state.raw<{ name: string; type: string; embedded: boolean }[]>([]);
  let fontPage = $state(0);

  $effect(() => {
    const d = doc;
    fonts = [];
    fontPage = 0;
    if (tab !== "fonts" || !d) return;
    let live = true;
    void (async () => {
      const seen = new Map<string, { name: string; type: string; embedded: boolean }>();
      try {
        for (let n = 1; n <= d.numPages && live; n++) {
          const p = await d.getPage(n);
          const ops = await p.getOperatorList();
          for (let i = 0; i < ops.fnArray.length; i++) {
            if (ops.fnArray[i] !== OPS.setFont) continue;
            const id = ops.argsArray[i][0] as string;
            if (seen.has(id) || !p.commonObjs.has(id)) continue;
            const f = p.commonObjs.get(id);
            seen.set(id, { name: String(f.name ?? id).replace(/^[A-Z]{6}\+/, ""), type: String(f.type ?? ""), embedded: !f.missingFile });
          }
          if (!live) return;
          fontPage = n;
          fonts = [...seen.values()].sort((a, b) => a.name.localeCompare(b.name));
        }
      } catch {
        // The document closed meanwhile.
      }
    })();
    return () => {
      live = false;
    };
  });

  onDestroy(() => {
    watcher?.disconnect();
  });
</script>

<div class="side">
  <div class="tabs" role="tablist">
    {#each TABS as [id, t, e] (id)}
      <button role="tab" aria-selected={tab === id} class:on={tab === id} title={l(t, e)} onclick={() => choose(id)}>{l(t, e)}</button>
    {/each}
  </div>
  <div class="body">
    {#if tab === "thumbs"}
      <div class="thumbs" bind:this={list}>
        {#each Array.from({ length: doc?.numPages ?? 0 }, (_, i) => i + 1) as n (n)}
          <button data-n={n} class:on={n === page} title={l(`Sayfa ${labels?.[n - 1] ?? n}`, `Page ${labels?.[n - 1] ?? n}`)} onclick={() => go(n)}>
            <span>{labels?.[n - 1] ?? n}</span>
          </button>
        {/each}
      </div>
    {:else if tab === "outline"}
      {#if outline?.length}
        <Outline entries={outline} {tr} {follow} />
      {:else}
        <p class="muted">{l("Bu PDF'te ana hat yok.", "This PDF has no outline.")}</p>
      {/if}
    {:else if tab === "info"}
      <dl>
        {#each info as [k, v] (k)}
          <dt>{k}</dt>
          <dd>{v}</dd>
        {/each}
      </dl>
    {:else}
      {#if doc && fontPage < doc.numPages}<p class="muted">{fontPage} / {doc.numPages}…</p>{/if}
      <ul>
        {#each fonts as f (f.name + f.type)}
          <li>{f.name} <span class="muted">{f.type}{f.embedded ? "" : l(", gömülü değil", ", not embedded")}</span></li>
        {/each}
      </ul>
    {/if}
  </div>
</div>

<style>
  .side {
    flex: 0 0 190px;
    display: flex;
    flex-direction: column;
    min-width: 0;
    min-height: 0;
    font-size: 12px;
    color: var(--chrome-fg);
    background: var(--chrome);
    border-right: 1px solid var(--chrome-line);
  }
  .tabs {
    display: flex;
    flex-wrap: wrap;
    border-bottom: 1px solid var(--chrome-line);
  }
  .tabs button {
    flex: 1;
    padding: 4px;
    font-size: 11px;
    color: inherit;
    background: none;
    border: 0;
    border-bottom: 2px solid transparent;
    cursor: pointer;
  }
  .tabs button.on {
    border-bottom-color: var(--claret);
  }
  button:focus-visible {
    outline: 2px solid var(--claret);
    outline-offset: -2px;
  }
  .body {
    flex: 1;
    min-height: 0;
    overflow: auto;
    display: flex;
    flex-direction: column;
  }
  .thumbs {
    flex: 1;
    min-height: 0;
    overflow: auto;
    padding: 8px;
  }
  .thumbs button {
    display: block;
    width: 138px;
    min-height: 184px;
    margin: 0 auto 8px;
    padding: 4px 4px 0;
    color: inherit;
    font: inherit;
    background: none;
    border: 1px solid transparent;
    border-radius: 3px;
    cursor: pointer;
  }
  .thumbs button.on {
    border-color: var(--claret);
    background: var(--chrome-2);
  }
  .thumbs button :global(canvas) {
    display: block;
    width: 100%;
    background: white;
    box-shadow: 0 1px 4px rgba(0, 0, 0, 0.25);
  }
  .muted {
    margin: 8px;
    color: var(--chrome-muted);
  }
  li .muted {
    margin: 0;
  }
  dl {
    margin: 8px;
  }
  dt {
    color: var(--chrome-muted);
  }
  dd {
    margin: 0 0 6px;
    overflow-wrap: anywhere;
  }
  ul {
    margin: 8px;
    padding: 0;
    list-style: none;
  }
  li {
    margin-bottom: 5px;
    overflow-wrap: anywhere;
  }
</style>
