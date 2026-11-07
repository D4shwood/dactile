// The pages in their scroller. Every page has an empty sheet of the right
// size, so 300 pages cost 300 divs; only the sheets near the view hold a
// canvas and a text layer, and the far ones give theirs back.

import { TextLayer, type PDFDocumentProxy, type PDFPageProxy, type RenderTask } from "pdfjs-dist";
import type { PageViewport } from "pdfjs-dist";
import { fitAcross, fitDown, PAD, placeOf, screenPages, screenStart, UNIT, type Grid } from "./layout";

/// No canvas larger than this: past it, pages are drawn coarser and stretched.
const MAX_PIXELS = 16_000_000;

interface Sheet {
  n: number;
  el: HTMLDivElement;
  /// Size in points, as turned.
  w: number;
  h: number;
  page?: PDFPageProxy;
  canvas?: HTMLCanvasElement;
  /// What the canvas was drawn for.
  drawn: string;
  task?: RenderTask;
  text?: TextLayer;
  /// The viewport at 100 %.
  base?: PageViewport;
}

export interface Place {
  page: number;
  /// How far down the page the view's top is, 0 to 1.
  down: number;
  left: number;
}

export interface Spot {
  page: number;
  /// In PDF points, from the page's bottom left.
  x: number;
  y: number;
  /// In PDF points, from the page's top left, as SyncTeX counts.
  top: number;
}

export type Fit = "" | "width" | "text" | "window";

/// A rectangle laid over a page, in PDF points.
export interface Mark {
  rect: number[];
  kind: "hit" | "now" | "flash";
}

export interface Events {
  page: (n: number) => void;
  zoom: (percent: number) => void;
  error: (text: string) => void;
}

export class Sheets {
  private grid: HTMLDivElement;
  private doc: PDFDocumentProxy | null = null;
  private opening: PDFDocumentProxy | null = null;
  /// Which `arrange` is the latest.
  private arranging = 0;
  private sheets: Sheet[] = [];
  private marks = new Map<number, Mark[]>();
  private flashed = new Map<number, Mark[]>();
  private fading = 0;
  private sharpening: RenderTask | null = null;
  private busy = false;
  private frame = 0;
  private later = 0;
  private drawing: Sheet | null = null;
  private watch: ResizeObserver;
  private lay: Grid = { columns: 1, rows: 1, offset: 0, single: false };
  private continuous = true;
  private turn = 0;
  private fitting: Fit = "width";
  /// The first page shown, when pages come a screen at a time.
  private first = 1;
  zoom = 100;
  page = 1;

  constructor(
    private host: HTMLElement,
    private on: Events,
  ) {
    this.grid = document.createElement("div");
    this.grid.className = "grid";
    host.replaceChildren(this.grid);
    host.addEventListener("scroll", this.scrolled, { passive: true });
    this.watch = new ResizeObserver(() => this.resized());
    this.watch.observe(host);
  }

  get pages() {
    return this.sheets.length;
  }

  get document() {
    return this.doc;
  }

  /// `doc` in place of the one shown, at the same place: its pages appear
  /// at once, empty, and fill from the view outwards.
  async open(doc: PDFDocumentProxy) {
    this.opening = doc;
    const one = await doc.getPage(1);
    if (this.opening !== doc) return;
    const keep = this.sheets.length ? this.place() : undefined;
    this.stop();
    this.doc = doc;
    const size = this.sizeOf(one);
    this.grid.replaceChildren();
    this.sheets = [];
    for (let n = 1; n <= doc.numPages; n++) {
      const el = document.createElement("div");
      el.className = "sheet";
      el.dataset.page = String(n);
      this.grid.append(el);
      this.sheets.push({ n, el, w: size.w, h: size.h, drawn: "" });
    }
    this.sheets[0].page = one;
    this.page = Math.min(keep?.page ?? 1, doc.numPages);
    this.first = screenStart(this.page, this.lay);
    await this.arrange(keep && { ...keep, page: this.page });
    void this.measure(doc);
  }

  /// A document about to be destroyed, if it is the one shown.
  forget(doc: PDFDocumentProxy) {
    if (this.doc === doc) this.close();
  }

  /// Nothing shown: drawing stops, and the document may be destroyed.
  close() {
    this.opening = null;
    this.stop();
    this.doc = null;
    this.sheets = [];
    this.grid.replaceChildren();
  }

  destroy() {
    this.close();
    this.watch.disconnect();
    this.host.removeEventListener("scroll", this.scrolled);
    cancelAnimationFrame(this.frame);
    clearTimeout(this.later);
  }

  private stop() {
    for (const s of this.sheets) this.drop(s);
  }

  /// Plays have pages of one size; when one differs, the sheets follow,
  /// after the first ones are on the screen.
  private async measure(doc: PDFDocumentProxy) {
    let changed = false;
    for (const s of this.sheets) {
      if (this.doc !== doc) return;
      const held = s.page;
      try {
        const page = held ?? (await doc.getPage(s.n));
        if (this.doc !== doc) return;
        const { w, h } = this.sizeOf(page);
        if (!held) page.cleanup();
        if (Math.abs(w - s.w) > 0.5 || Math.abs(h - s.h) > 0.5) {
          s.w = w;
          s.h = h;
          changed = true;
        }
      } catch {
        return;
      }
    }
    if (changed) this.arrange();
  }

  private sizeOf(page: PDFPageProxy) {
    const v = this.viewOf(page, 1);
    return { w: v.width, h: v.height };
  }

  private viewOf(page: PDFPageProxy, scale: number) {
    return page.getViewport({ scale, rotation: (page.rotate + this.turn) % 360 });
  }

  /// Grid, continuity and turn, all at once. Keeps the place.
  set(g: Grid, continuous: boolean, turn: number) {
    const keep = this.place();
    if (turn !== this.turn) {
      for (const s of this.sheets) {
        if ((turn - this.turn) % 180) [s.w, s.h] = [s.h, s.w];
        this.drop(s);
      }
    }
    this.lay = g;
    this.continuous = continuous;
    this.turn = turn;
    this.first = screenStart(this.page, g);
    void this.arrange(keep);
  }

  /// A fit stays: it is worked out again when the window changes or a new
  /// PDF comes.
  fit(mode: Fit) {
    this.fitting = mode;
    void this.arrange();
  }

  /// A zoom in percent, keeping the point at (x, y) of the view in place.
  setZoom(percent: number, x?: number, y?: number) {
    this.fitting = "";
    const px = x ?? this.host.clientWidth / 2;
    const py = y ?? this.host.clientHeight / 2;
    const ratio = percent / this.zoom;
    const sx = this.host.scrollLeft + px;
    const sy = this.host.scrollTop + py;
    this.zoom = percent;
    this.size();
    this.host.scrollLeft = sx * ratio - px;
    this.host.scrollTop = sy * ratio - py;
    this.on.zoom(percent);
    this.redraw();
  }

  private widest() {
    return this.shown().reduce((m, s) => Math.max(m, s.w), 1);
  }

  private tallest() {
    return this.shown().reduce((m, s) => Math.max(m, s.h), 1);
  }

  /// What the screen holds: every page in a continuous view.
  private shown() {
    if (this.continuous) return this.sheets;
    const [a, b] = screenPages(this.first, this.pages, this.lay);
    return this.sheets.slice(a - 1, b);
  }

  private fitted(box: [number, number] | null): number {
    const { clientWidth: w, clientHeight: h } = this.host;
    const across = fitAcross(w, this.widest(), this.lay.columns);
    if (this.fitting === "window") return Math.min(across, fitDown(h, this.tallest(), this.lay.rows));
    if (box) return fitAcross(w, box[1] - box[0] + 12, this.lay.columns);
    return across;
  }

  /// How wide the text runs, left and right, over the page in view and the
  /// next two, in points at 100 %.
  private async textBox(): Promise<[number, number] | null> {
    const doc = this.doc;
    if (!doc) return null;
    let left = Infinity;
    let right = -Infinity;
    for (let n = this.page; n < this.page + 3 && n <= doc.numPages; n++) {
      const page = await doc.getPage(n);
      if (this.doc !== doc) return null;
      const view = this.viewOf(page, 1);
      for (const item of (await page.getTextContent()).items) {
        if (!("str" in item) || !item.str.trim()) continue;
        const [x, y] = [item.transform[4], item.transform[5]];
        for (const [a, b] of [
          [x, y],
          [x + item.width, y],
        ]) {
          const p = view.convertToViewportPoint(a, b);
          left = Math.min(left, p[0]);
          right = Math.max(right, p[0]);
        }
      }
    }
    return left < right ? [left, right] : null;
  }

  /// Lays the sheets out at the zoom or fit asked for, the view staying at
  /// `keep`.
  private async arrange(keep = this.place()) {
    const turn = ++this.arranging;
    this.layout();
    const box = this.fitting === "text" ? await this.textBox() : null;
    if (turn !== this.arranging) return;
    if (this.fitting && this.sheets.length) {
      this.zoom = this.fitted(box);
      this.on.zoom(this.zoom);
    }
    this.size();
    this.scrollTo(keep);
    if (box) this.host.scrollLeft = PAD - 6 + (box[0] * this.zoom * UNIT) / 100;
    this.update();
  }

  private scrollTo(p: Place) {
    const s = this.sheets[p.page - 1];
    if (!s) return;
    this.host.scrollTop = s.el.offsetTop + p.down * s.el.offsetHeight;
    this.host.scrollLeft = p.left;
  }

  /// Which sheets show, and how they sit.
  private layout() {
    const g = this.lay;
    this.grid.style.gridTemplateColumns = `repeat(${g.columns}, max-content)`;
    const [a, b] = this.continuous ? [1, this.pages] : screenPages(this.first, this.pages, g);
    for (const s of this.sheets) {
      s.el.hidden = s.n < a || s.n > b;
      s.el.style.gridColumnStart = "";
    }
    const start = this.sheets[a - 1];
    if (start) {
      const place = this.continuous || !g.single ? placeOf(a, g) : 0;
      if (place % g.columns) start.el.style.gridColumnStart = String((place % g.columns) + 1);
    }
  }

  private size() {
    const f = (this.zoom / 100) * UNIT;
    this.grid.style.setProperty("--scale-factor", String(f));
    for (const s of this.sheets) {
      s.el.style.width = `${Math.round(s.w * f)}px`;
      s.el.style.height = `${Math.round(s.h * f)}px`;
    }
  }

  private resized() {
    if (this.fitting) void this.arrange();
    else this.update();
  }

  /// A zoom settles before pages are drawn again: till then they stretch.
  private redraw() {
    clearTimeout(this.later);
    this.later = window.setTimeout(() => this.update(), 120);
  }

  private scrolled = () => {
    cancelAnimationFrame(this.frame);
    this.frame = requestAnimationFrame(() => this.update());
  };

  private key() {
    return `${this.zoom.toFixed(2)}/${this.turn}`;
  }

  /// The sheets in view, then those a screen around them. Beyond three
  /// screens, pages go.
  private update() {
    if (!this.sheets.length) return;
    // A page being drawn for a zoom that has passed.
    if (this.drawing && this.drawing.drawn !== this.key()) this.drawing.task?.cancel();
    const top = this.host.scrollTop;
    const height = this.host.clientHeight;
    const middle = top + height / 2;
    let best: Sheet | null = null;
    let nearest = Infinity;
    for (const s of this.sheets) {
      if (s.el.hidden) {
        this.drop(s);
        continue;
      }
      const a = s.el.offsetTop;
      const b = a + s.el.offsetHeight;
      const off = middle < a ? a - middle : middle > b ? middle - b : 0;
      if (off < nearest) {
        nearest = off;
        best = s;
      }
      if (b < top - 3 * height || a > top + 4 * height) this.drop(s);
    }
    if (best && best.n !== this.page) {
      this.page = best.n;
      this.on.page(best.n);
    }
    void this.pump();
  }

  /// The next sheet to draw: the nearest to the view that needs it.
  private next(): Sheet | null {
    const top = this.host.scrollTop;
    const height = this.host.clientHeight;
    const middle = top + height / 2;
    const key = this.key();
    let best: Sheet | null = null;
    let nearest = Infinity;
    for (const s of this.sheets) {
      if (s.el.hidden || s.drawn === key) continue;
      const a = s.el.offsetTop;
      const b = a + s.el.offsetHeight;
      if (b < top - height || a > top + 2 * height) continue;
      const off = Math.abs((a + b) / 2 - middle);
      if (off < nearest) {
        nearest = off;
        best = s;
      }
    }
    return best;
  }

  /// One page at a time, so a new view is never behind old work.
  private async pump() {
    if (this.busy) return;
    this.busy = true;
    try {
      for (let s = this.next(); s; s = this.next()) await this.draw(s);
    } finally {
      this.busy = false;
    }
  }

  private async draw(s: Sheet) {
    const doc = this.doc;
    if (!doc) return;
    const key = this.key();
    // Whatever comes of it, this page isn't tried again for this view.
    s.drawn = key;
    try {
      const page = s.page ?? (await doc.getPage(s.n));
      if (this.doc !== doc || s.drawn !== key) return;
      s.page = page;
      const view = this.viewOf(page, (this.zoom / 100) * UNIT);
      s.base = this.viewOf(page, 1);
      const ratio = Math.min(devicePixelRatio || 1, Math.sqrt(MAX_PIXELS / (view.width * view.height)));
      const canvas = document.createElement("canvas");
      canvas.width = Math.floor(view.width * ratio);
      canvas.height = Math.floor(view.height * ratio);
      const task = page.render({
        canvasContext: canvas.getContext("2d")!,
        viewport: view,
        transform: ratio === 1 ? undefined : [ratio, 0, 0, ratio, 0, 0],
      });
      s.task = task;
      this.drawing = s;
      await task.promise;
      s.task = undefined;
      if (this.doc !== doc || s.drawn !== key) return void (canvas.width = 0);
      if (s.canvas) {
        s.canvas.replaceWith(canvas);
        s.canvas.width = 0;
      } else s.el.prepend(canvas);
      s.canvas = canvas;
      await this.layers(s, page, view);
    } catch (e) {
      s.task = undefined;
      const name = (e as Error)?.name;
      if (name !== "RenderingCancelledException" && name !== "AbortException" && this.doc === doc) {
        this.on.error(String(e));
      }
    } finally {
      if (this.drawing === s) this.drawing = null;
    }
  }

  /// What lies over the canvas: the text to select.
  private async layers(s: Sheet, page: PDFPageProxy, view: PageViewport) {
    this.paint(s);
    if (s.text) return;
    const div = document.createElement("div");
    div.className = "textLayer";
    s.el.append(div);
    const text = new TextLayer({ textContentSource: page.streamTextContent(), container: div, viewport: view });
    s.text = text;
    await text.render();
  }

  /// The search's marks, by page, in place of those there.
  mark(marks: Map<number, Mark[]>) {
    this.marks = marks;
    for (const s of this.sheets) this.paint(s);
  }

  /// Marks are in percent of the page, so a zoom leaves them right.
  private paint(s: Sheet) {
    s.el.querySelector(".marks")?.remove();
    const list = [...(this.marks.get(s.n) ?? []), ...(this.flashed.get(s.n) ?? [])];
    if (!list.length || !s.base) return;
    const layer = document.createElement("div");
    layer.className = "marks";
    for (const m of list) {
      const [a, b, c, d] = s.base.convertToViewportRectangle(m.rect);
      const div = document.createElement("div");
      div.className = m.kind;
      div.style.left = `${(Math.min(a, c) / s.base.width) * 100}%`;
      div.style.top = `${(Math.min(b, d) / s.base.height) * 100}%`;
      div.style.width = `${(Math.abs(c - a) / s.base.width) * 100}%`;
      div.style.height = `${(Math.abs(d - b) / s.base.height) * 100}%`;
      layer.append(div);
    }
    s.canvas ? s.canvas.after(layer) : s.el.prepend(layer);
  }

  /// Brings a rectangle of a page into view, a third of the way down. With
  /// `flash`, the rectangles light up for a moment.
  async reveal(n: number, rect: number[], flash: number[][] = []) {
    const doc = this.doc;
    const s = this.sheets[n - 1];
    if (!doc || !s) return;
    const view = this.viewOf(await doc.getPage(n), 1);
    if (this.doc !== doc) return;
    const [a, b, c, d] = view.convertToViewportRectangle(rect);
    const f = (this.zoom / 100) * UNIT;
    this.show(n);
    this.host.scrollTop = s.el.offsetTop + Math.min(b, d) * f - this.host.clientHeight / 3;
    const left = s.el.offsetLeft + Math.min(a, c) * f;
    const right = s.el.offsetLeft + Math.max(a, c) * f;
    if (left < this.host.scrollLeft || right > this.host.scrollLeft + this.host.clientWidth) this.host.scrollLeft = left - PAD;
    this.page = n;
    this.on.page(n);
    if (flash.length) {
      clearTimeout(this.fading);
      const old = [...this.flashed.keys()];
      this.flashed = new Map([[n, flash.map((r) => ({ rect: r, kind: "flash" as const }))]]);
      s.base ??= view;
      for (const p of [...old, n]) if (this.sheets[p - 1]) this.paint(this.sheets[p - 1]);
      this.fading = window.setTimeout(() => {
        this.flashed = new Map();
        this.paint(s);
      }, 2500);
    }
    this.update();
  }

  /// Where a page's top is, in PDF points: SyncTeX counts down from it.
  async pageTop(n: number): Promise<number> {
    const page = await this.doc!.getPage(n);
    return page.view[3];
  }

  /// The drawn page under a point of the window.
  under(x: number, y: number) {
    for (const s of this.sheets) {
      if (s.el.hidden || !s.canvas) continue;
      const rect = s.el.getBoundingClientRect();
      if (x >= rect.left && x <= rect.right && y >= rect.top && y <= rect.bottom) return { n: s.n, rect, canvas: s.canvas };
    }
    return null;
  }

  /// A page drawn `factor` times finer than on the screen, for the lens.
  async sharp(n: number, factor: number): Promise<HTMLCanvasElement | null> {
    const page = this.sheets[n - 1]?.page;
    if (!page) return null;
    const view = this.viewOf(page, (this.zoom / 100) * UNIT * factor);
    const ratio = Math.min(devicePixelRatio || 1, Math.sqrt(MAX_PIXELS / (view.width * view.height)));
    const canvas = document.createElement("canvas");
    canvas.width = Math.floor(view.width * ratio);
    canvas.height = Math.floor(view.height * ratio);
    const task = page.render({
      canvasContext: canvas.getContext("2d")!,
      viewport: view,
      transform: ratio === 1 ? undefined : [ratio, 0, 0, ratio, 0, 0],
    });
    this.sharpening = task;
    try {
      await task.promise;
      return canvas;
    } catch {
      canvas.width = 0;
      return null;
    } finally {
      if (this.sharpening === task) this.sharpening = null;
    }
  }

  stopSharp() {
    this.sharpening?.cancel();
  }

  /// A far page gives back all it holds.
  private drop(s: Sheet) {
    if (!s.drawn && !s.page && !s.canvas) return;
    s.task?.cancel();
    s.task = undefined;
    s.text?.cancel();
    s.text = undefined;
    if (s.canvas) s.canvas.width = 0;
    s.canvas = undefined;
    s.el.replaceChildren();
    s.drawn = "";
    s.page?.cleanup();
    s.page = undefined;
  }

  /// Where the view is, to come back to.
  place(): Place {
    const s = this.sheets[this.page - 1];
    if (!s) return { page: 1, down: 0, left: 0 };
    const height = s.el.offsetHeight || 1;
    return {
      page: s.n,
      down: (this.host.scrollTop - s.el.offsetTop) / height,
      left: this.host.scrollLeft,
    };
  }

  restore(p: Place) {
    if (!this.sheets[p.page - 1]) return;
    this.show(p.page);
    this.scrollTo(p);
    this.page = p.page;
    this.on.page(p.page);
    this.update();
  }

  /// Brings the page's screen up, when pages come a screen at a time.
  private show(n: number) {
    if (this.continuous) return;
    this.first = screenStart(n, this.lay);
    this.layout();
  }

  /// Goes to a page, its top in view; `y` is points from its top, at 100 %.
  go(n: number, y = 0) {
    const s = this.sheets[Math.min(Math.max(n, 1), this.pages) - 1];
    if (!s) return;
    this.show(s.n);
    this.host.scrollTop = s.el.offsetTop - PAD + (y * this.zoom * UNIT) / 100;
    this.page = s.n;
    this.on.page(s.n);
    this.update();
  }

  /// The place under a point of the window, if on a page.
  at(x: number, y: number): Spot | null {
    for (const s of this.sheets) {
      if (s.el.hidden) continue;
      const r = s.el.getBoundingClientRect();
      if (x < r.left || x > r.right || y < r.top || y > r.bottom || !s.base) continue;
      const k = s.base.width / r.width;
      const [px, py] = s.base.convertToPdfPoint((x - r.left) * k, (y - r.top) * k);
      const box = s.base.viewBox;
      return { page: s.n, x: px, y: py, top: box[3] - py };
    }
    return null;
  }

  /// Zooms so a rectangle of the window fills the view.
  zoomTo(left: number, top: number, right: number, bottom: number) {
    const r = this.host.getBoundingClientRect();
    const k = Math.min(r.width / (right - left), r.height / (bottom - top));
    const midX = (left + right) / 2 - r.left;
    const midY = (top + bottom) / 2 - r.top;
    const z = Math.min(400, Math.max(12.5, this.zoom * k));
    this.setZoom(z, midX, midY);
    this.host.scrollLeft += midX - r.width / 2;
    this.host.scrollTop += midY - r.height / 2;
  }
}
