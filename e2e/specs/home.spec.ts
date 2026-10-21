import { expect, test } from "@playwright/test";
import { apiHeartbeat, apiProject, apiSend } from "../lib/api";
import { authFile } from "../lib/roles";

test.use({ storageState: authFile("tl") });

test("All incidents searches, filters, keeps the query, and the incident leads back to the same list", async ({ page }) => {
  const name = `Ledger ${Date.now()}`;
  const slug = await apiProject(page, name);
  const { pingUrl } = await apiHeartbeat(page, slug);
  expect((await page.request.post(`${pingUrl}/fail`)).status()).toBe(200);

  await page.goto("/incidents");
  await page.getByLabel("Search incidents").fill(name);
  const row = page.getByRole("link", { name: "nightly-settlement reported a failure" });
  await expect(row).toHaveCount(1);
  await expect(page).toHaveURL(/[?&]q=/);

  await page.getByRole("combobox", { name: "Alert level" }).click();
  await page.getByRole("option", { name: "Warning" }).click();
  await expect(page.getByText("No incidents match these filters.")).toBeVisible();
  await page.getByRole("combobox", { name: "Alert level" }).click();
  await page.getByRole("option", { name: "Critical" }).click();

  await row.click();
  await expect(page.getByRole("heading", { name: "nightly-settlement reported a failure" })).toBeVisible();
  await page.getByRole("link", { name: "All incidents" }).click();
  await expect(page.getByLabel("Search incidents")).toHaveValue(name);

  // A bare visit later reopens the same query.
  await page.goto("/");
  await page.goto("/incidents");
  await expect(page).toHaveURL(/[?&]q=Ledger/);
  await expect(row).toHaveCount(1);
});

test("All tickets opens a ticket over the list and closes back to it; a tech lead has no Admin link", async ({ page }) => {
  const name = `Kiosk ${Date.now()}`;
  const slug = await apiProject(page, name);
  const { ref_id } = await apiSend<{ ref_id: string }>(page, "POST", `/api/ui/projects/${slug}/ref-ids`, { single_use: false }, 201);
  await apiSend(page, "POST", "/api/v1/tickets", { ref_id, title: "Receipt prints twice", description: "Every sale prints two receipts." }, 201);

  await page.goto("/tickets");
  await expect(page.getByRole("link", { name: "Admin", exact: true })).toHaveCount(0);
  await page.getByLabel("Search tickets").fill(name);
  await page.getByRole("button", { name: "Receipt prints twice" }).click();
  await expect(page.getByRole("dialog").getByText("Every sale prints two receipts.")).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(page.getByRole("dialog")).toHaveCount(0);
  await expect(page).toHaveURL(/\/tickets(\?|$)/);
});
