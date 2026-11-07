<script lang="ts">
  // The built PDF beside the page, drawn by pdf.js: the bars, the keys and
  // the mouse. The pages themselves are in pdf/sheets.ts.
  import { onDestroy, onMount, tick, untrack } from "svelte";
  import * as pdfjs from "pdfjs-dist";
  import worker from "pdfjs-dist/build/pdf.worker.min.mjs?url";
  import { api, type BuildResult, type SyncPlace, type Viewer } from "./api";
  import BottomBar from "./pdf/BottomBar.svelte";
  import Clock from "./pdf/Clock.svelte";
  import Icon from "./pdf/Icon.svelte";
  import { centimetres, clampZoom, MAX_COPIES, printProblem, stepOf, stepZoom, Trail, type Mode } from "./pdf/layout";
  import { Lens } from "./pdf/lens";
  import Search from "./pdf/Search.svelte";
  import { Sheets, type Fit, type Place } from "./pdf/sheets";
  import Side from "./pdf/Side.svelte";
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
    goto: (line: number, quiet?: boolean) => void;
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
  let doc = $state.raw<pdfjs.PDFDocumentProxy | null>(null);
  let full = $state(false);
  let present = $state(false);
  let clock = $state(false);
  let laser = $state<{ x: number; y: number } | null>(null);
  // An empty box is null.
  let printing = $state<{ first: number | null; last: number | null; copies: number | null } | null>(null);
  let printNote = $state("");
  let size = $state(0);
  let labels = $state.raw<string[] | null>(null);
  let search: Search | undefined = $state();
  // The lens while the button is held: where it is, and what draws in it.
  let glass = $state<{ x: number; y: number } | null>(null);
  let glassCanvas: HTMLCanvasElement | undefined = $state();
  let lens: Lens | null = null;

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
    // pdf.js takes the bytes over: their length is gone after.
    const length = bytes.length;
    const loading = pdfjs.getDocument({ data: bytes });
    try {
      const opened = await loading.promise;
      if (n !== drawn || !sheets) return;
      failure = "";
      await sheets.open(opened);
      // A newer build came meanwhile: this one is about to close.
      if (n !== drawn) return sheets.forget(opened);
      pages = opened.numPages;
      size = length;
      labels = await opened.getPageLabels();
      doc = opened;
      if (waiting) void reveal(...waiting);
      waiting = null;
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
    kept = { ...v };
    sheets = new Sheets(host, {
      page: (n) => (page = n),
      zoom: (z) => (zoom = z),
      error: (text) => (failure = text),
      link: follow,
    });
    apply();
    sheets.fit(v.fit);
    if (!v.fit) sheets.setZoom(clampZoom(v.zoom));
    pinching(host);
  });

  // A drawing under way stops, and the document closes.
  onDestroy(() => {
    drawn++;
    endLens();
    if (full || present) api.fullScreen(false).catch(() => {});
    sheets?.destroy();
    release?.();
    release = null;
    clearTimeout(following);
    // A change made just before closing is kept too.
    if (saving) {
      clearTimeout(saving);
      save();
    }
  });

  // ------------------------------------------------------- SyncTeX

  // A line asked for before the PDF was on the screen.
  let waiting: [number, number] | null = null;
  let lastLine = 0;
  let following = 0;
  // When the view last moved by itself: that isn't the reader scrolling.
  let moved = 0;

  /// From the writer: the place of a line, lit up for a moment.
  export async function reveal(line: number, column: number, quiet = false) {
    if (!doc || !sheets) {
      waiting = [line, column];
      return;
    }
    try {
      const places = await api.syncView(line, column);
      const first = places[0];
      const top = await sheets.pageTop(first.page);
      const rect = (p: SyncPlace) => [p.x, top - p.y - p.height, p.x + p.width, top - p.y];
      moved = Date.now();
      await sheets.reveal(first.page, rect(first), places.filter((p) => p.page === first.page).map(rect));
    } catch (e) {
      if (!quiet) say(String(e), true);
    }
  }

  /// The writer's cursor moved: followed, if the choice is on.
  export function cursorMoved(line: number, column: number) {
    if (!o?.follow_cursor || line === lastLine) return;
    lastLine = line;
    clearTimeout(following);
    following = window.setTimeout(() => reveal(line, column, true), 350);
  }

  /// To the writer: the line behind a place of the PDF.
  async function toSource(page: number, x: number, top: number, quiet = false) {
    if (result?.kind !== "built") return;
    try {
      goto(await api.syncEdit(result.pdf, page, x, top), quiet);
    } catch (e) {
      if (!quiet) say(String(e), true);
    }
  }

  /// The reader scrolled: the writer's cursor follows, if the choice is on.
  function onscroll() {
    if (!o?.follow_scroll || Date.now() - moved < 800 || !root?.contains(document.activeElement)) return;
    clearTimeout(following);
    following = window.setTimeout(() => {
      const r = host?.getBoundingClientRect();
      const s = r && sheets?.at(r.left + r.width / 2, r.top + r.height / 2);
      if (s) void toSource(s.page, s.x, s.top, true);
    }, 400);
  }

  // ------------------------------------------------------- the choices

  let saving = 0;
  /// The choices as last sent.
  let kept: Viewer | Record<string, never> = {};
  /// The choices kept, a moment after the last change.
  function keep() {
    clearTimeout(saving);
    saving = window.setTimeout(save, 400);
  }

  /// Only what changed goes out: another window has its own choices.
  function save() {
    saving = 0;
    if (!o) return;
    const now = $state.snapshot(o) as Viewer;
    const was = kept as Record<string, unknown>;
    const changed = Object.entries(now).filter(([k, v]) => v !== was[k]);
    if (!changed.length) return;
    kept = now;
    api.set("viewer", Object.fromEntries(changed)).catch((e) => say(String(e), true));
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

  /// The bottom bar's toggles: a panel opens at its last tab.
  function toggle(side: "left" | "right") {
    if (!o) return;
    if (side === "right") choose({ right: o.right ? "" : "search" });
    else choose({ left: o.left ? "" : "thumbs" });
  }

  // ------------------------------------------------------- the whole screen

  function setFull(on: boolean) {
    full = on;
    api.fullScreen(on).catch((e) => say(String(e), true));
  }

  /// One page at a time, on black. The grid and the fit chosen come back
  /// after.
  function startShow() {
    if (!sheets || !pages) return;
    menu = false;
    present = true;
    api.fullScreen(true).catch((e) => say(String(e), true));
    sheets.set({ columns: 1, rows: 1, offset: 0, single: true }, false, turn);
    sheets.fit("window");
    host?.focus();
  }

  function endShow() {
    present = false;
    laser = null;
    clock = false;
    if (!full) api.fullScreen(false).catch((e) => say(String(e), true));
    apply();
    if (!o) return;
    if (o.fit) sheets?.fit(o.fit);
    else sheets?.setZoom(clampZoom(o.zoom));
    host?.focus();
  }

  /// In a presentation: left next, right previous, the middle button the
  /// laser pointer while held.
  function showDown(e: PointerEvent) {
    e.preventDefault();
    if (e.button === 0) sheets?.go(page + 1);
    else if (e.button === 2) sheets?.go(page - 1);
    else if (e.button === 1) {
      laser = { x: e.clientX, y: e.clientY };
      drag(
        e,
        (m) => (laser = { x: m.clientX, y: m.clientY }),
        () => (laser = null),
      );
    }
  }

  function showKey(e: KeyboardEvent): boolean {
    const keys: Record<string, () => void> = {
      Escape: endShow,
      ArrowRight: () => sheets?.go(page + 1),
      ArrowDown: () => sheets?.go(page + 1),
      PageDown: () => sheets?.go(page + 1),
      " ": () => sheets?.go(page + 1),
      ArrowLeft: () => sheets?.go(page - 1),
      ArrowUp: () => sheets?.go(page - 1),
      PageUp: () => sheets?.go(page - 1),
      Home: () => sheets?.go(1),
      End: () => sheets?.go(pages),
      c: () => (clock = !clock),
      C: () => (clock = !clock),
    };
    const f = keys[e.key];
    if (!f) return false;
    e.preventDefault();
    e.stopPropagation();
    f();
    return true;
  }

  // ------------------------------------------------------- printing

  function openPrint() {
    menu = false;
    printing = { first: 1, last: pages, copies: 1 };
    printNote = "";
  }

  /// A double click is one print.
  let sending = false;

  async function print() {
    const p = printing;
    if (!p || sending || result?.kind !== "built") return;
    const notes = {
      first: l(`İlk sayfa 1 ile ${pages} arasında bir tam sayı olmalı.`, `The first page must be a whole number from 1 to ${pages}.`),
      last: l(`Son sayfa 1 ile ${pages} arasında bir tam sayı olmalı.`, `The last page must be a whole number from 1 to ${pages}.`),
      order: l("İlk sayfa son sayfadan büyük olamaz.", "The first page can't come after the last."),
      copies: l(`Kopya sayısı 1 ile ${MAX_COPIES} arasında bir tam sayı olmalı.`, `Copies must be a whole number from 1 to ${MAX_COPIES}.`),
    };
    const problem = printProblem(p.first, p.last, p.copies, pages);
    if (problem) return void (printNote = notes[problem]);
    sending = true;
    try {
      await api.pdfPrint(result.pdf, p.first!, p.last!, p.copies!);
      printing = null;
      say(l("Yazıcıya gönderildi.", "Sent to the printer."));
    } catch (e) {
      printNote = String(e);
    }
    sending = false;
  }

  /// A quarter turn, for the session: the file stays as it is.
  function rotate(by: number) {
    turn = (turn + by + 360) % 360;
    apply();
  }

  /// Ctrl+F: the search panel, ready to type in.
  async function openSearch() {
    if (!o) return;
    if (o.right !== "search") choose({ right: "search" });
    await tick();
    search?.focus();
  }

  async function findAgain(against: boolean) {
    if (!o) return;
    if (o.right !== "search") return openSearch();
    search?.again(against);
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

  /// A link or an outline entry. Web addresses go to the browser.
  async function follow(dest: unknown, url?: string) {
    if (url) {
      if (!/^(https?:|mailto:)/i.test(url)) return say(l("Bu bağlantı açılmaz.", "This link can't be opened."), true);
      return api.openUrl(url).catch((e) => say(String(e), true));
    }
    try {
      const to = await sheets?.follow(dest);
      if (to) jump(() => void sheets?.reveal(to.page, to.rect));
    } catch (e) {
      say(String(e), true);
    }
  }

  /// The page box takes a page's number, or its label if the PDF has them.
  function pageOf(text: string): number {
    const t = text.trim();
    const i = labels?.indexOf(t) ?? -1;
    return i >= 0 ? i + 1 : parseInt(t, 10);
  }

  const step = () => (o ? stepOf({ columns: o.columns, rows: o.rows, offset: o.offset, single: o.single_step }) : 1);

  function goPage(text: string) {
    const n = pageOf(text);
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
    if (present && showKey(e)) return;
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
    if (mod && e.shiftKey && (e.key === "f" || e.key === "F")) return take(() => setFull(!full));
    if (mod && !e.shiftKey) {
      if (e.key === "=" || e.key === "+") return take(() => setZoom(stepZoom(zoom, true)));
      if (e.key === "-") return take(() => setZoom(stepZoom(zoom, false)));
      if (e.key === "1") return take(() => setZoom(100));
      if (e.key === "2") return take(() => fit("width"));
      if (e.key === "3") return take(() => fit("window"));
      if (e.key === "4") return take(() => fit("text"));
      if (e.key === "j" || e.key === "J") return take(gotoBox);
      if (e.key === "f" || e.key === "F") return take(openSearch);
      if (e.key === "Home") return take(() => act("first"));
      if (e.key === "End") return take(() => act("last"));
      return;
    }
    if (e.key === "Escape") return take(escape);
    if (e.key === "F3" && !mod) return take(() => findAgain(e.shiftKey));
    if (e.key === "F5" && !mod) return take(startShow);
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
    if (glass) endLens();
    else if (printing) printing = null;
    else if (menu) menu = false;
    else if (band) band = null;
    else if (full) setFull(false);
    else if (o?.right) {
      choose({ right: "" });
      host?.focus();
    } else if (o?.left) {
      choose({ left: "" });
      host?.focus();
    } else if (wide) wide = false;
    else close();
  }

  function endLens() {
    lens?.end();
    lens = null;
    glass = null;
  }

  /// The lens opens under the pointer and follows it while the button is
  /// held.
  async function openLens(e: PointerEvent) {
    if (!o || !sheets?.under(e.clientX, e.clientY)) return;
    const point = (m: PointerEvent) => {
      if (lens) glass = { x: m.clientX, y: m.clientY };
      lens?.move(m.clientX, m.clientY);
    };
    glass = { x: e.clientX, y: e.clientY };
    drag(e, point, endLens);
    await tick();
    if (!glassCanvas || !glass) return;
    lens = new Lens(sheets, glassCanvas, o.lens_size);
    lens.move(glass.x, glass.y);
  }

  // ------------------------------------------------------- the mouse

  function onwheel(e: WheelEvent) {
    if (!e.ctrlKey) return;
    e.preventDefault();
    const r = host!.getBoundingClientRect();
    setZoom(zoom * (e.deltaY < 0 ? 1.1 : 1 / 1.1), e.clientX - r.left, e.clientY - r.top);
  }

  // Pinching on a touch screen; WebKit's gesture events where they exist.
  let pinchFrom = 0;
  let pinchZoom = 100;
  const apart = (t: TouchList) => Math.hypot(t[0].clientX - t[1].clientX, t[0].clientY - t[1].clientY);

  function pinching(el: HTMLElement) {
    el.addEventListener("touchstart", (e) => {
      if (e.touches.length !== 2) return;
      pinchFrom = apart(e.touches);
      pinchZoom = zoom;
    });
    el.addEventListener(
      "touchmove",
      (e) => {
        if (e.touches.length !== 2 || !pinchFrom) return;
        e.preventDefault();
        setZoom((pinchZoom * apart(e.touches)) / pinchFrom);
      },
      { passive: false },
    );
    el.addEventListener("touchend", () => (pinchFrom = 0));
    el.addEventListener("gesturestart", (e) => {
      e.preventDefault();
      pinchZoom = zoom;
    });
    el.addEventListener("gesturechange", (e) => {
      e.preventDefault();
      setZoom(pinchZoom * (e as unknown as { scale: number }).scale);
    });
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
    // A pointer already gone cannot be captured: the drag still ends on its "up".
    try {
      el.setPointerCapture(e.pointerId);
    } catch {}
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
    if (present) return showDown(e);
    if (e.button !== 0 || !sheets || !host) return;
    if (e.target === host && e.offsetX > host.clientWidth) return; // Its scroll bar.
    const mod = e.ctrlKey || e.metaKey;
    // A plain click on a link follows it, in any mode.
    if (!mod && !e.shiftKey && !e.altKey && (e.target as HTMLElement).closest?.("a.link")) return;
    if (mod && e.shiftKey) {
      e.preventDefault();
      const s = sheets.at(e.clientX, e.clientY);
      if (s) void copyPlace(s.x, s.y);
      return;
    }
    if (mod) {
      e.preventDefault();
      const s = sheets.at(e.clientX, e.clientY);
      if (s) void toSource(s.page, s.x, s.top);
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
    if (!e.shiftKey) return void openLens(e);
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

  async function copyPlace(x: number, y: number) {
    const text = centimetres(x, y);
    try {
      await navigator.clipboard.writeText(text);
    } catch {
      // WebKitGTK may refuse the clipboard API: the old way, as the writer does.
      const t = document.createElement("textarea");
      t.value = text;
      root?.append(t);
      t.select();
      const ok = document.execCommand("copy");
      t.remove();
      host?.focus();
      if (!ok) return say(l("Pano kullanılamadı.", "The clipboard can't be used."), true);
    }
    say(l(`Konum kopyalandı: ${text} cm`, `Position copied: ${text} cm`));
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
        <div class="row">
          <button class="word" title={l("Tam ekran (Ctrl+Shift+F)", "Full screen (Ctrl+Shift+F)")} onclick={() => setFull(!full)}>{l("Tam ekran", "Full screen")}</button>
          <button class="word" title={l("Sunum (F5, odak görüntüleyicideyken)", "Presentation (F5, with the focus in the viewer)")} disabled={none} onclick={startShow}>{l("Sunum", "Presentation")}</button>
          <button class="word" title={l("Yazdır", "Print")} disabled={none} onclick={openPrint}>{l("Yazdır", "Print")}</button>
        </div>
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
        <div class="row">
          <span>{l("Döndür", "Rotate")}</span>
          <button class="word" title={l("Sola döndür", "Rotate left")} onclick={() => rotate(-90)}>↺</button>
          <button class="word" title={l("Sağa döndür", "Rotate right")} onclick={() => rotate(90)}>↻</button>
        </div>
        <label title={l("Renkleri ters çevir", "Invert the colours")}><input type="checkbox" checked={o.invert} onchange={(e) => choose({ invert: e.currentTarget.checked })} /> {l("Renkleri çevir", "Invert colours")}</label>
        <label title={l("Gri tonlarda göster", "Show in grayscale")}><input type="checkbox" checked={o.gray} onchange={(e) => choose({ gray: e.currentTarget.checked })} /> {l("Gri tonlama", "Grayscale")}</label>
        <hr />
        <label title={l("PDF yazarın imlecini izler", "The PDF follows the writer's cursor")}><input type="checkbox" checked={o.follow_cursor} onchange={(e) => choose({ follow_cursor: e.currentTarget.checked })} /> {l("Kaydırma imleci izlesin", "Scrolling follows cursor")}</label>
        <label title={l("Yazarın imleci PDF'i izler", "The writer's cursor follows the PDF")}><input type="checkbox" checked={o.follow_scroll} onchange={(e) => choose({ follow_scroll: e.currentTarget.checked })} /> {l("İmleç kaydırmayı izlesin", "Cursor follows scrolling")}</label>
        <hr />
        <label title={l("Yuvarlak mercek; kapalıysa dikdörtgen", "A round lens; off: a rectangle")}><input type="checkbox" checked={o.lens_round} onchange={(e) => choose({ lens_round: e.currentTarget.checked })} /> {l("Yuvarlak mercek", "Round lens")}</label>
        <label title={l("Merceğin gölgesi", "The lens's shadow")}><input type="checkbox" checked={o.lens_shadow} onchange={(e) => choose({ lens_shadow: e.currentTarget.checked })} /> {l("Gölge", "Shadow")}</label>
        <label title={l("Merceğin kenarlığı", "The lens's border")}><input type="checkbox" checked={o.lens_border} onchange={(e) => choose({ lens_border: e.currentTarget.checked })} /> {l("Kenarlık", "Border")}</label>
        <div class="row">
          <span>{l("Mercek boyu", "Lens size")}</span>
          <input type="number" min="80" max="400" step="10" value={o.lens_size} title={l("Merceğin çapı, piksel", "The lens's size, in pixels")} aria-label={l("Mercek boyu", "Lens size")} onchange={(e) => choose({ lens_size: Math.min(400, Math.max(80, +e.currentTarget.value || 170)) })} />
        </div>
      </div>
    {/if}
  </span>
{/snippet}

<!-- svelte-ignore a11y_no_noninteractive_tabindex, a11y_no_noninteractive_element_interactions -->
<aside class="pdf" class:wide={wide || full || present} class:present bind:this={root} tabindex="-1" onkeydown={onkey} onmouseup={onmouseup}>
  {#if present}
    <!-- No bars in a presentation. -->
  {:else if o?.toolbar}
    <TopBar {tr} bind:mode bind:box fit={o.fit} {page} {pages} label={labels?.[page - 1] ?? ""} {canBack} {canForward} {wide} disabled={none} {act} go={goPage} {more} />
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
  <div class="middle">
    {#if o?.left && !present}
      <Side {tr} tab={o.left} {doc} {page} bytes={size} {labels} choose={(tab) => choose({ left: tab })} go={(n) => jump(() => sheets?.go(n))} {follow} />
    {/if}
    <!-- svelte-ignore a11y_no_noninteractive_tabindex, a11y_no_static_element_interactions -->
    <div class="pages {mode}" class:invert={o?.invert} class:gray={o?.gray} class:dim={building} bind:this={host} tabindex="0" onpointerdown={ondown} {onwheel} {onscroll} oncontextmenu={(e) => present && e.preventDefault()}></div>
    {#if o?.right && sheets && !present}
      <div class="panel">
        <div class="tabs" role="tablist">
          <button role="tab" aria-selected={o.right === "search"} class:on={o.right === "search"} title={l("Ara (Ctrl+F)", "Search (Ctrl+F)")} onclick={() => choose({ right: "search" })}>{l("Ara", "Search")}</button>
          <button role="tab" aria-selected={o.right === "clock"} class:on={o.right === "clock"} title={l("Saat", "Clock")} onclick={() => choose({ right: "clock" })}>{l("Saat", "Clock")}</button>
        </div>
        {#if o.right === "search"}
          <Search bind:this={search} {tr} {doc} {page} {sheets} {say} />
        {:else}
          <Clock {tr} />
        {/if}
      </div>
    {/if}
  </div>
  {#if !present}
    <BottomBar {tr} hasLeft left={!!o?.left} right={!!o?.right} {page} {pages} {zoom} disabled={none} {toggle} {setZoom} step={(up) => setZoom(stepZoom(zoom, up))} />
  {/if}
  {#if laser}
    <div class="laser" style:left="{laser.x}px" style:top="{laser.y}px"></div>
  {/if}
  {#if present && clock}
    <div class="showclock"><Clock {tr} /></div>
  {/if}
  {#if printing}
    <div class="print" role="dialog" aria-label={l("Yazdır", "Print")}>
      <strong>{l("Yazdır", "Print")}</strong>
      <label>{l("İlk sayfa", "First page")} <input type="number" min="1" max={pages} bind:value={printing.first} /></label>
      <label>{l("Son sayfa", "Last page")} <input type="number" min="1" max={pages} bind:value={printing.last} /></label>
      <label>{l("Kopya", "Copies")} <input type="number" min="1" max={MAX_COPIES} bind:value={printing.copies} /></label>
      {#if printNote}<p class="note-print" role="alert">{printNote}</p>{/if}
      <div class="row">
        <button class="word" title={l("Varsayılan yazıcıya gönder", "Send to the default printer")} onclick={print}>{l("Yazdır", "Print")}</button>
        <button class="word" title={l("Vazgeç (Esc)", "Cancel (Esc)")} onclick={() => (printing = null)}>{l("Vazgeç", "Cancel")}</button>
      </div>
    </div>
  {/if}
  {#if glass && o}
    <div
      class="lens"
      class:round={o.lens_round}
      class:shadow={o.lens_shadow}
      class:border={o.lens_border}
      class:invert={o.invert}
      class:gray={o.gray}
      style:left="{glass.x - o.lens_size / 2}px"
      style:top="{glass.y - o.lens_size / 2}px"
      style:width="{o.lens_size}px"
      style:height="{o.lens_size}px"
    >
      <canvas bind:this={glassCanvas}></canvas>
    </div>
  {/if}
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
  .pdf.present {
    background: #000;
  }
  .present .pages {
    overflow: hidden;
    cursor: default;
  }
  .present .pages :global(.sheet) {
    box-shadow: none;
  }
  .laser {
    position: fixed;
    z-index: 8;
    width: 14px;
    height: 14px;
    margin: -7px 0 0 -7px;
    border-radius: 50%;
    pointer-events: none;
    background: #ff2a2a;
    box-shadow: 0 0 12px 4px rgba(255, 42, 42, 0.6);
  }
  .showclock {
    position: fixed;
    right: 16px;
    bottom: 16px;
    z-index: 8;
    border-radius: 8px;
    background: var(--chrome);
  }
  .print {
    position: absolute;
    top: 40px;
    left: 50%;
    z-index: 7;
    transform: translateX(-50%);
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: 10px 14px;
    font-size: 12px;
    color: var(--chrome-fg);
    background: var(--chrome);
    border: 1px solid var(--chrome-line);
    border-radius: 6px;
    box-shadow: 0 6px 18px rgba(0, 0, 0, 0.3);
  }
  .note-print {
    max-width: 240px;
    margin: 0;
    color: var(--claret);
  }
  .print label {
    display: flex;
    justify-content: space-between;
    gap: 10px;
  }
  .print input {
    width: 4em;
    font: inherit;
    color: inherit;
    background: var(--chrome-2);
    border: 1px solid var(--chrome-line);
    border-radius: 3px;
    caret-color: var(--accent-ink);
  }
  .print .word {
    padding: 2px 10px;
    font: inherit;
    color: inherit;
    background: none;
    border: 1px solid var(--chrome-line);
    border-radius: 4px;
    cursor: pointer;
  }
  .print .word:focus-visible {
    outline: 2px solid var(--accent-ink);
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
  .drop hr {
    width: 100%;
    margin: 2px 0;
    border: 0;
    border-top: 1px solid var(--chrome-line);
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
    caret-color: var(--accent-ink);
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
  .middle {
    flex: 1;
    display: flex;
    min-height: 0;
  }
  .panel {
    flex: 0 0 200px;
    display: flex;
    flex-direction: column;
    min-height: 0;
    color: var(--chrome-fg);
    background: var(--chrome);
    border-left: 1px solid var(--chrome-line);
  }
  .tabs {
    display: flex;
    flex-wrap: wrap;
    border-bottom: 1px solid var(--chrome-line);
  }
  .tabs button {
    flex: 1;
    padding: 4px 6px;
    font-size: 12px;
    color: inherit;
    background: none;
    border: 0;
    border-bottom: 2px solid transparent;
    cursor: pointer;
  }
  .tabs button.on {
    border-bottom-color: var(--accent-ink);
  }
  .tabs button:focus-visible {
    outline: 2px solid var(--accent-ink);
    outline-offset: -2px;
  }
  .lens {
    position: fixed;
    z-index: 7;
    overflow: hidden;
    pointer-events: none;
    background: white;
  }
  .lens.round {
    border-radius: 50%;
  }
  .lens.shadow {
    box-shadow: 0 6px 24px rgba(0, 0, 0, 0.4);
  }
  .lens.border {
    outline: 2px solid var(--accent-ink);
  }
  .lens canvas {
    display: block;
    width: 100%;
    height: 100%;
  }
  .lens.invert canvas {
    filter: invert(1) hue-rotate(180deg);
  }
  .lens.gray canvas {
    filter: grayscale(1);
  }
  .lens.invert.gray canvas {
    filter: invert(1) grayscale(1);
  }
  .pages :global(.sheet a.link) {
    position: absolute;
    z-index: 2;
    cursor: pointer;
  }
  .pages :global(.sheet a.link:focus-visible) {
    outline: 2px solid var(--accent-ink);
  }
  .pages :global(.marks) {
    position: absolute;
    inset: 0;
    pointer-events: none;
  }
  .pages :global(.marks div) {
    position: absolute;
    border-radius: 2px;
    mix-blend-mode: multiply;
  }
  .pages :global(.marks .hit) {
    background: #ffd84d;
  }
  .pages :global(.marks .now) {
    background: #ff9f43;
  }
  .pages :global(.marks .flash) {
    background: color-mix(in srgb, var(--claret) 35%, transparent);
    animation: flash 2.5s ease-out forwards;
  }
  @keyframes flash {
    70% {
      opacity: 1;
    }
    100% {
      opacity: 0;
    }
  }
  .pages {
    flex: 1;
    min-width: 0;
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
  /* On the page, white in every theme. */
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
    border-left: 3px solid var(--accent-ink);
    background: var(--chrome);
    border-radius: 4px;
  }
  .error p {
    white-space: pre-wrap;
    font: 12px/1.4 var(--mono);
    margin: 6px 0;
  }
  .error button {
    border: 1px solid var(--accent-ink);
    color: var(--accent-ink);
    background: none;
    border-radius: 4px;
    padding: 3px 10px;
    cursor: pointer;
  }
</style>
