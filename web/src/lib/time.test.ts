import { expect, test } from "bun:test";
import { ago, day, duration, stamp, inputValue, fromInputValue } from "@/lib/time";

test("formats times in the offset the way the server does", () => {
  expect(stamp(0, 0)).toBe("1970-01-01 00:00 UTC");
  expect(stamp(1_792_161_874, 0)).toBe("2026-10-16 14:44 UTC");
  expect(stamp(1_792_161_874, 420)).toBe("2026-10-16 21:44 UTC+7");
  expect(stamp(1_792_161_874, 330)).toBe("2026-10-16 20:14 UTC+5:30");
  expect(stamp(1_792_108_800 + 3600, -180)).toBe("2026-10-15 22:00 UTC-3");
  expect(day(1_792_108_800 + 3600, -180)).toBe("2026-10-15");
});

test("formats durations and ages the way the server does", () => {
  expect(duration(378)).toBe("6m 18s");
  expect(duration(7_440)).toBe("2h 4m");
  expect(duration(-5)).toBe("0s");
  expect(ago(100, 100 + 360)).toBe("6m ago");
  expect(ago(100, 110)).toBe("just now");
  expect(ago(0, 3 * 86_400 + 5)).toBe("3d ago");
});

test("a datetime input reads and writes times in the admin's offset", () => {
  const ts = Date.UTC(2026, 9, 16, 8, 0) / 1000;
  expect(inputValue(ts, 420)).toBe("2026-10-16T15:00");
  expect(fromInputValue("2026-10-16T15:00", 420)).toBe(ts);
  expect(fromInputValue(inputValue(ts, -180), -180)).toBe(ts);
  expect(fromInputValue("", 0)).toBeNull();
});
