import { expect, test } from "bun:test";
import { compareVersions, RELEASES, releaseUrl, showsWhatsNew } from "@/lib/whatsNew";

test("versions compare part by part as numbers", () => {
  expect(compareVersions("1.10.0", "1.9.3")).toBe(1);
  expect(compareVersions("1.1.0", "1.1.0")).toBe(0);
  expect(compareVersions("1.0.1", "1.1")).toBe(-1);
});

test("the banner shows once per release", () => {
  expect(showsWhatsNew("1.1.0", "")).toBe(true);
  expect(showsWhatsNew("1.1.0", "1.0.1")).toBe(true);
  expect(showsWhatsNew("1.1.0", "1.1.0")).toBe(false);
  expect(showsWhatsNew("1.1.0", "1.2.0")).toBe(false);
});

test("releases run newest first and link to their GitHub notes", () => {
  for (let i = 1; i < RELEASES.length; i++) expect(compareVersions(RELEASES[i - 1]!.version, RELEASES[i]!.version)).toBe(1);
  expect(releaseUrl("1.1.0")).toBe("https://github.com/PT-Perkasa-Pilar-Utama/monita/releases/tag/v1.1.0");
});
