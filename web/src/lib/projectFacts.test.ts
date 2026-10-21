import { expect, test } from "bun:test";
import { allowlistSize, publicTabs } from "@/lib/projectFacts";

test("Uptime always leads the public tabs that are on", () => {
  expect(publicTabs({ incidents: false, known_issues: true, report: true, changelog: false })).toEqual(["Uptime", "Issues", "Ticket"]);
});

test("an empty allowlist counts zero entries", () => {
  expect(allowlistSize(null)).toBe(0);
  expect(allowlistSize("10.0.0.0/8\n\n192.168.1.5")).toBe(2);
});
