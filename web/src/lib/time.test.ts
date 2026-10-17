import { expect, test } from "bun:test";
import { ago, day, duration, utc } from "@/lib/time";

test("formats UTC times the way the server does", () => {
  expect(utc(0)).toBe("1970-01-01 00:00 UTC");
  expect(utc(1_792_161_874)).toBe("2026-10-16 14:44 UTC");
  expect(day(1_792_137_600)).toBe("2026-10-16");
});

test("formats durations and ages the way the server does", () => {
  expect(duration(378)).toBe("6m 18s");
  expect(duration(7_440)).toBe("2h 4m");
  expect(duration(-5)).toBe("0s");
  expect(ago(100, 100 + 360)).toBe("6m ago");
  expect(ago(100, 110)).toBe("just now");
  expect(ago(0, 3 * 86_400 + 5)).toBe("3d ago");
});
