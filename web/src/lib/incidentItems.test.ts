import { describe, expect, test } from "bun:test";
import { pickItems } from "@/lib/incidentItems";
import type { IncidentItem } from "@/lib/types";

const items: IncidentItem[] = [
  { value: "monitor:1", kind: "monitor", name: "API health", state: "down" },
  { value: "monitor:2", kind: "monitor", name: "Checkout", state: null },
  { value: "heartbeat:1", kind: "heartbeat", name: "Nightly API sync", state: null },
];

describe("pickItems", () => {
  test("an empty search splits every item by kind", () => {
    const { monitors, heartbeats } = pickItems(items, " ");
    expect(monitors.map((i) => i.value)).toEqual(["monitor:1", "monitor:2"]);
    expect(heartbeats.map((i) => i.value)).toEqual(["heartbeat:1"]);
  });
  test("a search keeps names that contain it, ignoring case", () => {
    const { monitors, heartbeats } = pickItems(items, "api");
    expect(monitors.map((i) => i.value)).toEqual(["monitor:1"]);
    expect(heartbeats.map((i) => i.value)).toEqual(["heartbeat:1"]);
  });
  test("names sort with their numbers in order", () => {
    const api = (n: number): IncidentItem => ({ value: `monitor:${n}`, kind: "monitor", name: `api ${n}`, state: null });
    const { monitors } = pickItems([api(11), api(2), { ...api(1), name: "API 1" }], "");
    expect(monitors.map((i) => i.name)).toEqual(["API 1", "api 2", "api 11"]);
  });
  test("no match leaves both lists empty", () => {
    expect(pickItems(items, "billing")).toEqual({ monitors: [], heartbeats: [] });
  });
});
