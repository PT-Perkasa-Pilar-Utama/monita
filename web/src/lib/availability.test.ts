import { expect, test } from "bun:test";
import { barWidth, grade, pct } from "@/lib/availability";

test("availability grades at three and two nines", () => {
  expect(grade(99.95)).toBe("good");
  expect(grade(99.9)).toBe("good");
  expect(grade(99.5)).toBe("fair");
  expect(grade(98)).toBe("poor");
  expect(grade(null)).toBe("none");
});

test("the bar starts at 90 percent and a missing figure reads as a dash", () => {
  expect(barWidth(95)).toBe(50);
  expect(barWidth(80)).toBe(0);
  expect(barWidth(100)).toBe(100);
  expect(pct(null)).toBe("–");
  expect(pct(99.98765)).toBe("99.988%");
});
