import { expect, test } from "@playwright/test";
import { authFile } from "../lib/roles";

test.use({ storageState: authFile("tl") });

test("@US-23 @US-24 @US-25 @US-26 @US-28 @US-29 a reported bug goes public only after the TL triages it", async ({ page, browser }) => {
  const slug = `t-${Date.now()}`;
  await page.goto("/");
  await page.getByLabel("Name").fill("Shop");
  await page.getByLabel("Slug").fill(slug);
  await page.getByRole("button", { name: "Create project" }).click();
  await page.goto(`/projects/${slug}/settings`);
  await page.getByLabel("Key name").fill("backend");
  await page.getByRole("button", { name: "Create key" }).click();
  const key = (await page.locator("#new-key").textContent())!.trim();

  // The project's backend issues a ref ID; the reporter submits from the project's own UI.
  const issued = await page.request.post("/api/v1/ref-ids", { headers: { authorization: `Bearer ${key}` }, data: {} });
  const refId = (await issued.json()).data.ref_id as string;
  const visitor = await browser.newContext({ storageState: { cookies: [], origins: [] } });
  const v = await visitor.newPage();
  const created = await v.request.post("/api/v1/tickets", {
    data: { ref_id: refId, title: "Card of Budi declined", description: "Card 4111 declined at the last step" },
  });
  expect(created.status()).toBe(201);

  await v.goto(`/status/${slug}/tickets`);
  await expect(v.getByText("Card of Budi declined")).toHaveCount(0);

  await page.goto(`/projects/${slug}/tickets`);
  await page.getByRole("link", { name: /Card of Budi declined/ }).click();
  await page.getByLabel("Public title").fill("A card is declined at checkout");
  await page.getByRole("button", { name: "Save public title" }).click();
  await expect(page.getByText("Outside Triage the public board shows it as A card is declined at checkout")).toBeVisible();
  await page.getByLabel("Column").selectOption("in_progress");
  await page.getByRole("button", { name: "Move ticket" }).click();

  // The board is cacheable for 30 s (spec 10.1); a fresh URL skips the visitor's cached copy.
  await v.goto(`/status/${slug}/tickets?after=move`);
  await expect(v.getByText("A card is declined at checkout")).toBeVisible();
  await expect(v.getByText("Budi")).toHaveCount(0);
  await expect(v.getByText("4111")).toHaveCount(0);

  await v.goto(`/status/${slug}/mine`);
  await v.getByLabel("Ref ID").fill(refId);
  await v.getByRole("button", { name: "Show my tickets" }).click();
  await expect(v.getByText("Card of Budi declined")).toBeVisible();
  await expect(v.getByText(/In progress/)).toBeVisible();
  await visitor.close();
});
