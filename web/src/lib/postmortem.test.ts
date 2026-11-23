import { expect, test } from "bun:test";
import { publishNote, SECTIONS } from "@/lib/postmortem";

test("the sections follow Google's post-mortem and three are needed to publish", () => {
  expect(SECTIONS.map((s) => s.key)).toEqual(["summary", "impact", "root_cause", "trigger", "resolution", "detection", "action_items", "lessons"]);
  expect(SECTIONS.filter((s) => s.required).map((s) => s.key)).toEqual(["summary", "root_cause", "resolution"]);
});

test("the publish dialog says who gets the email", () => {
  expect(publishNote(0)).toContain("no email goes out");
  expect(publishNote(1)).toContain("the 1 address");
  expect(publishNote(3)).toContain("the 3 addresses");
});
