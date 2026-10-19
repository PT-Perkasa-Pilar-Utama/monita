import { expect, test } from "@playwright/test";
import { apiProject, apiSend } from "../lib/api";
import { authFile } from "../lib/roles";

test.use({ storageState: authFile("tl") });

const release = (version: string, released_on: string, title: string): object => ({ version, released_on, title, added: "One thing.", changed: "", fixed: "", removed: "" });

test("a release is written one change per field, and the public tab walks the versions", async ({ page }) => {
  const slug = await apiProject(page, "Changelog");
  await page.goto(`/projects/${slug}/changelog`);
  await page.getByRole("button", { name: "New release" }).click();
  await page.locator("#r-version").fill("latest");
  await page.locator("#r-day").fill("2026-10-01");
  await page.locator("#r-title").fill("First release");
  await page.getByLabel("Added change 1", { exact: true }).fill("Exports in CSV");
  await page.getByLabel("Added change 1", { exact: true }).press("Enter");
  await page.getByLabel("Added change 2", { exact: true }).fill("A dark theme");
  await page.getByRole("button", { name: "Save release" }).click();
  await expect(page.locator("[data-slot=field-error]").first()).toBeVisible();
  await page.locator("#r-version").fill("1.0.0");
  await page.getByRole("button", { name: "Save release" }).click();
  await expect(page.getByText("Release 1.0.0 saved.")).toBeVisible();

  await apiSend(page, "POST", `/api/ui/projects/${slug}/changelog`, release("1.1.0", "2026-10-15", "Second release"), 201);
  await apiSend(page, "PUT", `/api/ui/projects/${slug}/public-pages`, { incidents: true, known_issues: true, report: true, changelog: true }, 204);

  await page.goto(`/status/${slug}/changelog`);
  await expect(page.getByText("Second release").first()).toBeVisible();
  await page.getByRole("link", { name: /Older 1\.0\.0/ }).click();
  await expect(page).toHaveURL(new RegExp(`/status/${slug}/changelog/1\\.0\\.0$`));
  await expect(page.getByText("Exports in CSV")).toBeVisible();
  await expect(page.getByText("A dark theme")).toBeVisible();

  await page.getByLabel("Find a release").fill("Second");
  const index = page.getByRole("navigation", { name: "Releases" });
  await expect(index.getByText("1.1.0")).toBeVisible();
  await expect(index.getByText("1.0.0")).toHaveCount(0);
});
