// The magnifier: at once from the page's canvas, then from a finer drawing
// of the page, made while the button is held and thrown away after.

import type { Sheets } from "./sheets";

export const LENS_ZOOM = 2;

export class Lens {
  private fine = new Map<number, HTMLCanvasElement | null>();
  private x = 0;
  private y = 0;
  private done = false;

  constructor(
    private sheets: Sheets,
    private canvas: HTMLCanvasElement,
    private size: number,
  ) {
    canvas.width = canvas.height = Math.round(size * (devicePixelRatio || 1));
  }

  move(x: number, y: number) {
    this.x = x;
    this.y = y;
    const under = this.sheets.under(x, y);
    if (!under) return;
    const source = this.fine.get(under.n) ?? under.canvas;
    const k = source.width / under.rect.width;
    const half = (this.size / LENS_ZOOM / 2) * k;
    const cx = (x - under.rect.left) * k;
    const cy = (y - under.rect.top) * k;
    const ctx = this.canvas.getContext("2d")!;
    ctx.fillStyle = "#fff";
    ctx.fillRect(0, 0, this.canvas.width, this.canvas.height);
    ctx.drawImage(source, cx - half, cy - half, half * 2, half * 2, 0, 0, this.canvas.width, this.canvas.height);
    if (!this.fine.has(under.n)) void this.sharpen(under.n);
  }

  private async sharpen(n: number) {
    this.fine.set(n, null);
    const canvas = await this.sheets.sharp(n, LENS_ZOOM);
    if (this.done) {
      if (canvas) canvas.width = 0;
      return;
    }
    this.fine.set(n, canvas);
    this.move(this.x, this.y);
  }

  end() {
    this.done = true;
    this.sheets.stopSharp();
    for (const c of this.fine.values()) if (c) c.width = 0;
    this.fine.clear();
  }
}
