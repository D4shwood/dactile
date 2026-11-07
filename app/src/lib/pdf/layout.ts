// The viewer's sums, with no page or window in them.

/// One CSS pixel for 1/96 inch: at 100 % the page is its printed size.
export const UNIT = 96 / 72;
export const PAD = 16;
export const GAP = 14;

export const MIN_ZOOM = 12.5;
export const MAX_ZOOM = 400;
export const STOPS = [12.5, 25, 33, 50, 67, 75, 100, 125, 150, 200, 300, 400];

export const clampZoom = (z: number) => Math.min(MAX_ZOOM, Math.max(MIN_ZOOM, z));

/// The next stop up or down from any zoom.
export function stepZoom(z: number, up: boolean): number {
  const next = up ? STOPS.find((s) => s > z + 0.01) : [...STOPS].reverse().find((s) => s < z - 0.01);
  return next ?? clampZoom(z);
}

export type Mode = "magnify" | "scroll" | "select";

export interface Grid {
  columns: number;
  rows: number;
  offset: number;
  single: boolean;
}

/// Next and previous move this many pages.
export const stepOf = (g: Grid) => (g.single ? 1 : g.columns * g.rows);

/// Where a page sits among the grid's places: the offset leaves some empty.
export const placeOf = (n: number, g: Grid) => n - 1 + g.offset;

/// The first page of the screen `n` is on, when pages come a screen at a
/// time.
export function screenStart(n: number, g: Grid): number {
  if (g.single) return n;
  const per = g.columns * g.rows;
  return Math.max(1, Math.floor(placeOf(n, g) / per) * per - g.offset + 1);
}

/// The pages of the screen starting at `first`, as [first, last].
export function screenPages(first: number, total: number, g: Grid): [number, number] {
  const per = g.columns * g.rows;
  const used = g.single ? 0 : placeOf(first, g) % per;
  return [first, Math.min(total, first + per - used - 1)];
}

/// The zoom at which `across` page widths of `width` fit `room`, in percent.
export function fitAcross(room: number, width: number, columns: number): number {
  const free = room - 2 * PAD - (columns - 1) * GAP;
  return clampZoom((100 * free) / columns / (width * UNIT));
}

/// The zoom at which `rows` pages of `height` fit `room`, in percent.
export function fitDown(room: number, height: number, rows: number): number {
  const free = room - 2 * PAD - (rows - 1) * GAP;
  return clampZoom((100 * free) / rows / (height * UNIT));
}

/// Where a click is, in centimetres from the page's bottom left, from PDF
/// points.
export function centimetres(x: number, y: number): string {
  const cm = (p: number) => ((p / 72) * 2.54).toFixed(2);
  return `${cm(x)} ${cm(y)}`;
}

export const MAX_COPIES = 99;

/// What is wrong with a print dialog's numbers, if anything. An empty box
/// comes as null.
export function printProblem(first: unknown, last: unknown, copies: unknown, pages: number): "" | "first" | "last" | "order" | "copies" {
  const whole = (n: unknown, top: number) => Number.isInteger(n) && (n as number) >= 1 && (n as number) <= top;
  if (!whole(first, pages)) return "first";
  if (!whole(last, pages)) return "last";
  if ((first as number) > (last as number)) return "order";
  return whole(copies, MAX_COPIES) ? "" : "copies";
}

/// Back and forward through the places jumped from.
export class Trail<T> {
  past: T[] = [];
  ahead: T[] = [];

  /// A jump from `here`: what lay ahead is gone.
  jump(here: T) {
    this.past.push(here);
    if (this.past.length > 100) this.past.shift();
    this.ahead = [];
  }
  back(here: T): T | undefined {
    const to = this.past.pop();
    if (to !== undefined) this.ahead.push(here);
    return to;
  }
  forward(here: T): T | undefined {
    const to = this.ahead.pop();
    if (to !== undefined) this.past.push(here);
    return to;
  }
}
