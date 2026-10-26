import { expect, test } from "bun:test";
import { columnAt, readout } from "@/lib/chart";
import type { Chart } from "@/lib/types";

const mark = (x: number, w: number, title: string) => ({ x, y: 10, w, h: 202, title });
const chart: Chart = {
  line: "",
  area: "",
  dots: [],
  up: [mark(100, 300, "Up")],
  down: [mark(400, 200, "Down for 3h 0m")],
  held: [],
  fails: [],
  oks: [],
  hovers: [mark(56, 448, "2026-10-19 14:00 UTC+7: No pings"), mark(504, 448, "2026-10-19 15:00 UTC+7: 2 pings, 1 failed")],
  y_ticks: [],
  x_ticks: [],
  empty: false,
};

test("the readout names the bucket's time, its count and whether it was down", () => {
  expect(readout(chart, 520)).toEqual({
    bucket: { x: 504, w: 448 },
    when: "2026-10-19 15:00 UTC+7",
    what: "2 pings, 1 failed",
    band: "Down for 3h 0m",
  });
  expect(readout(chart, 150)?.band).toBe("Up");
});

test("before the item existed there is no band, and outside the plot nothing", () => {
  expect(readout(chart, 60)?.band).toBeNull();
  expect(readout(chart, 20)).toBeNull();
  expect(readout(chart, 955)).toBeNull();
});

test("the Overview's readout picks the slot under the pointer", () => {
  const cols = [
    { x: 170, w: 10, label: "00:00" },
    { x: 180, w: 10, label: "00:15" },
  ];
  expect(columnAt(cols, 175)?.label).toBe("00:00");
  expect(columnAt(cols, 180)?.label).toBe("00:15");
  expect(columnAt(cols, 160)).toBeNull();
  expect(columnAt(cols, 190)).toBeNull();
});
