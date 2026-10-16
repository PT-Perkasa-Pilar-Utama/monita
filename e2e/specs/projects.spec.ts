import { expect, test } from "@playwright/test";
import { authFile } from "../lib/roles";

test.use({ storageState: authFile("tl") });

test("@US-06 @US-07 TL creates a project and a key that is shown once", async ({ page }) => {
  const slug = `p-${Date.now()}`;
  await page.goto("/");
  await page.getByLabel("Name").fill("Payments");
  await page.getByLabel("Slug").fill(slug);
  await page.getByRole("button", { name: "Create project" }).click();
  await expect(page).toHaveURL(new RegExp(`/projects/${slug}/monitors$`));
  await expect(page.getByRole("heading", { name: "No monitors yet" })).toBeVisible();

  await page.getByRole("link", { name: "Settings" }).click();
  await page.getByLabel("Key name").fill("backend-prod");
  await page.getByRole("button", { name: "Create key" }).click();
  const key = (await page.locator("#new-key").textContent())!.trim();
  expect(key).toMatch(/^mk_[a-z2-7]{26}$/);

  const ok = await page.request.post("/api/v1/ref-ids", { headers: { authorization: `Bearer ${key}` }, data: {} });
  expect(ok.status()).toBe(201);

  // A fresh visit: the key is never shown again.
  await page.goto(`/projects/${slug}/settings`);
  await expect(page.locator("#new-key")).toHaveCount(0);
  await page.getByRole("button", { name: "Revoke" }).click();
  const revoked = await page.request.post("/api/v1/ref-ids", { headers: { authorization: `Bearer ${key}` }, data: {} });
  expect(revoked.status()).toBe(401);
});
