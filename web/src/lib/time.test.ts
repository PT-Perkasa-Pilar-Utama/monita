import { expect, test } from "bun:test";
import { ago, day, digestNote, duration, stamp, inputValue, fromInputValue, windowHint } from "@/lib/time";

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

test("the digest note keeps the UTC hour and shows it in the chosen offset", () => {
  expect(digestNote(1, 0)).toBe("The daily ticket digest goes out at 01:00 UTC.");
  expect(digestNote(1, 420)).toBe("The daily ticket digest stays at 01:00 UTC, 08:00 in UTC+7.");
  expect(digestNote(1, -300)).toBe("The daily ticket digest stays at 01:00 UTC, 20:00 in UTC-5.");
});

test("a window hint says when it starts and flags a zone that is not this computer's", () => {
  const now = 1_000_000;
  expect(windowHint(now, now + 3600, now, 0, 0)).toBe("It starts now and lasts 1h 0m.");
  expect(windowHint(now + 7 * 3600, now + 8 * 3600, now, 0, 420)).toBe(
    "It starts in 7h 0m and lasts 1h 0m. Times here are in UTC, Monita's time zone, which is 7h behind your computer.",
  );
  expect(windowHint(now - 21, now + 3600, now, 0, 0)).toBe("It starts now and lasts 1h 0m.");
  expect(windowHint(null, null, now, 420, 0)).toBe("Times here are in UTC+7, Monita's time zone, which is 7h ahead of your computer.");
});
