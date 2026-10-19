import { expect, test } from "bun:test";
import { alertLine, allowlistSize, publicTabs } from "@/lib/projectFacts";
import type { AlertRoute } from "@/lib/types";

test("Uptime always leads the public tabs that are on", () => {
  expect(publicTabs({ incidents: false, known_issues: true, report: true, changelog: false })).toEqual(["Uptime", "Issues", "Ticket"]);
});

test("an alert line names who gets the alert, or says it is off", () => {
  const r: AlertRoute = { severity: "critical", emails: ["a@x.co", "b@x.co"], webhook_url: "https://hook", use_webhook: true, enabled: true };
  expect(alertLine(r)).toBe("Critical: 2 emails and the webhook.");
  expect(alertLine({ ...r, severity: "warning", emails: ["a@x.co"], use_webhook: false })).toBe("Warning: 1 email.");
  expect(alertLine({ ...r, emails: [], use_webhook: false })).toBe("Critical: no one is alerted.");
  expect(alertLine({ ...r, enabled: false })).toBe("Critical: off.");
});

test("an empty allowlist counts zero entries", () => {
  expect(allowlistSize(null)).toBe(0);
  expect(allowlistSize("10.0.0.0/8\n\n192.168.1.5")).toBe(2);
});
