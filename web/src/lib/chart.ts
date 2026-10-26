import type { Chart } from "@/lib/types";

/** The chart's plot runs from x 56 to 952 in a 960-wide view box. */
export const PLOT = { left: 56, right: 952, width: 960 };

export type Readout = {
  /** The bucket the pointer is on, as the server labels it. */
  bucket: { x: number; w: number };
  when: string;
  what: string;
  /** "Up", "Down for 10h 13m", or null before the item existed. */
  band: string | null;
};

const under = <T extends { x: number; w: number }>(marks: T[], x: number): T | undefined => marks.find((m) => x >= m.x && x < m.x + m.w);

/** What the pointer at view-box `x` is over: the bucket's time and count, and up or down. */
export function readout(chart: Chart, x: number): Readout | null {
  if (x < PLOT.left || x > PLOT.right) return null;
  const hover = under(chart.hovers, x);
  if (!hover) return null;
  const cut = hover.title.indexOf(": ");
  const band = under(chart.down, x) ?? under(chart.up, x);
  return {
    bucket: { x: hover.x, w: hover.w },
    when: cut < 0 ? hover.title : hover.title.slice(0, cut),
    what: cut < 0 ? "" : hover.title.slice(cut + 2),
    band: band?.title ?? null,
  };
}

/** The slot of the Overview chart under view-box `x`, or null outside the plot. */
export function columnAt<T extends { x: number; w: number }>(columns: T[], x: number): T | null {
  return columns.find((c) => x >= c.x && x < c.x + c.w) ?? null;
}
