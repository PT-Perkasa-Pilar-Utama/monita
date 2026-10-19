import { expect, test } from "@playwright/test";
import { apiProject } from "../lib/api";
import { authFile } from "../lib/roles";

test.use({ storageState: authFile("tl") });

test("each primitive's tab ends in its Moni footer; Overview and Settings have none", async ({ page }) => {
  const slug = await apiProject(page, "Footers");
  const footer = page.locator("footer svg[viewBox='0 0 560 200']");
  for (const tab of ["monitors", "heartbeats", "incidents", "tickets", "changelog"]) {
    await page.goto(`/projects/${slug}/${tab}`);
    await expect(footer, tab).toHaveCount(1);
  }
  for (const path of ["", "/settings"]) {
    await page.goto(`/projects/${slug}${path}`);
    await expect(page.getByRole("heading", { level: 1 })).toBeVisible();
    await expect(footer, path || "overview").toHaveCount(0);
  }
});
