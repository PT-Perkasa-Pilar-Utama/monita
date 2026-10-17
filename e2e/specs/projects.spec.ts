import { expect, test } from "@playwright/test";
import { createProject } from "../lib/project";
import { authFile } from "../lib/roles";

test.use({ storageState: authFile("tl") });

test("@US-06 @US-07 TL creates a project and a key that is shown once", async ({ page }) => {
  const slug = `p-${Date.now()}`;
  await createProject(page, "Payments", slug);
  await expect(page).toHaveURL(new RegExp(`/projects/${slug}/monitors$`));
  await expect(page.getByRole("heading", { name: "Add your first monitor" })).toBeVisible();

  await page.getByRole("link", { name: "Settings" }).click();
  await page.getByRole("link", { name: "API keys" }).click();
  await page.getByRole("button", { name: "New API key" }).click();
  await page.getByLabel("Key name").fill("backend-prod");
  await page.getByRole("button", { name: "Create key" }).click();
  const key = (await page.locator("#new-key").textContent())!.trim();
  expect(key).toMatch(/^mk_[a-z2-7]{26}$/);

  const ok = await page.request.post("/api/v1/ref-ids", { headers: { authorization: `Bearer ${key}` }, data: {} });
  expect(ok.status()).toBe(201);

  // A fresh visit: the key is never shown again.
  await page.goto(`/projects/${slug}/settings/keys`);
  await expect(page.locator("#new-key")).toHaveCount(0);
  await page.getByRole("button", { name: "Revoke" }).click();
  await page.getByRole("dialog").getByRole("button", { name: "Revoke" }).click();
  await expect(page.getByText("Revoked", { exact: true })).toBeVisible();
  const revoked = await page.request.post("/api/v1/ref-ids", { headers: { authorization: `Bearer ${key}` }, data: {} });
  expect(revoked.status()).toBe(401);
});
