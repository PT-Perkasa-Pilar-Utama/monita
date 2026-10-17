import { expect, test } from "@playwright/test";
import { createProject } from "../lib/project";
import { authFile } from "../lib/roles";

test.use({ storageState: authFile("tl") });

test("@US-06 @US-07 TL creates a project and a key that is shown once", async ({ page }) => {
  const slug = await createProject(page, "Payments");
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

test("@US-06 the page shrinks a picked icon before it reaches the server", async ({ page }) => {
  // A 1x1 PNG: the page draws it into a 128 px square and sends WebP.
  const png = Buffer.from("iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNkYPhfDwAChwGA60e6kgAAAABJRU5ErkJggg==", "base64");
  await page.goto("/");
  await page.getByRole("button", { name: "New project" }).click();
  const dialog = page.getByRole("dialog");
  await dialog.getByLabel("Name").fill("Branded");
  await dialog.getByLabel("Description (optional)").fill("The shop's storefront.");
  await dialog.getByLabel(/^Icon/).setInputFiles({ name: "logo.png", mimeType: "image/png", buffer: png });
  await expect(dialog.locator("[data-icon-preview]")).toBeVisible();
  await dialog.getByRole("button", { name: "Create project" }).click();
  await expect(page).toHaveURL(/\/monitors$/);
  const slug = new URL(page.url()).pathname.split("/")[2];

  const icon = await page.request.get(`/projects/${slug}/icon`);
  expect(icon.status()).toBe(200);
  expect(icon.headers()["content-type"]).toBe("image/webp");
  expect((await icon.body()).length).toBeLessThanOrEqual(32 * 1024);
  await page.goto("/");
  await expect(page.getByText("The shop's storefront.")).toBeVisible();
});
