import { expect, test } from "bun:test";
import { fillEmpty, publishNote, SECTIONS, template, unfilled } from "@/lib/postmortem";
import type { IncidentDetail, PostmortemText } from "@/lib/types";

function detail(over: Partial<IncidentDetail["incident"]> = {}, items = ["Checkout API"]): IncidentDetail {
  return {
    incident: { id: 4, origin: "auto", title: "Checkout API is down", severity: "critical", cause: "HTTP 503", started_at: 1_000_000, resolved_at: 1_003_600, last_label: "resolved", excluded: null, excluded_at: null, excluded_by: null, ...over },
    items,
    updates: [],
    labels: [],
  };
}

test("the sections follow Google's post-mortem and three are needed to publish", () => {
  expect(SECTIONS.map((s) => s.key)).toEqual(["summary", "impact", "root_cause", "trigger", "resolution", "detection", "action_items", "lessons"]);
  expect(SECTIONS.filter((s) => s.required).map((s) => s.key)).toEqual(["summary", "root_cause", "resolution"]);
});

test("the publish dialog says who gets the email", () => {
  expect(publishNote(0)).toContain("no email goes out");
  expect(publishNote(1)).toContain("the 1 address");
  expect(publishNote(3)).toContain("the 3 addresses");
});

test("a template states the incident's facts and leaves a prompt in every section", () => {
  const t = template(detail());
  expect(t.summary).toContain("INC-4: Checkout API is down. Checkout API was affected from");
  expect(t.summary).toContain("1h 0m");
  expect(t.root_cause).toStartWith("Monita saw: HTTP 503");
  expect(t.detection).toContain("Monita opened INC-4 at");
  expect(template(detail({ origin: "manual", cause: null }, [])).detection).toContain("was reported by hand");
  expect(template(detail({}, ["A", "B"])).summary).toContain("A, B were affected");
  expect(unfilled(t)).toEqual(SECTIONS.map((s) => s.key));
  for (const v of Object.values(t)) expect(v).toMatch(/^[ -~\n]*$/);
});

test("a long title, cause or list of systems keeps each section under the cap", () => {
  const t = template(detail({ title: "t".repeat(5_000), cause: "c".repeat(10_000) }, Array.from({ length: 500 }, (_, n) => `system-${n}`)));
  for (const v of Object.values(t)) expect(v.length).toBeLessThan(1_500);
});

test("filling keeps typed text, and a section is done once its prompts are replaced", () => {
  const empty = Object.fromEntries(SECTIONS.map((s) => [s.key, ""])) as PostmortemText;
  const filled = fillEmpty({ ...empty, summary: "Mine." }, template(detail()));
  expect(filled.summary).toBe("Mine.");
  expect(filled.impact).toContain("[Who was affected");
  const lessons = "What went well: alerts.\nWhat went wrong: slow rollback.\nWhere we got lucky: night time.";
  expect(unfilled({ ...filled, lessons, impact: "Card payments failed [for one bank]." })).not.toContain("lessons");
  expect(unfilled({ ...filled, impact: "Card payments failed [for one bank]." })).not.toContain("impact");
});
