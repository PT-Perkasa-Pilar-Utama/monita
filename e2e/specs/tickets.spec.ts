import { expect, test } from "@playwright/test";
import { createProject } from "../lib/project";
import { authFile } from "../lib/roles";

test.use({ storageState: authFile("tl") });

test("@US-23 @US-24 @US-25 @US-26 @US-28 @US-29 a reported bug goes public only after the TL triages it", async ({ page, browser }) => {
  const slug = await createProject(page, "Shop");
  await page.goto(`/projects/${slug}/settings/keys`);
  await page.getByRole("button", { name: "New API key" }).click();
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
  for (const column of ["Triage", "Backlog", "In progress", "Done"]) await expect(page.getByRole("heading", { name: column, level: 2 })).toBeVisible();
  await page.getByRole("link", { name: /Card of Budi declined/ }).click();
  await expect(page.getByText("Card 4111 declined at the last step")).toBeVisible();
  await page.getByLabel(/^Public title/).fill("A card is declined at checkout");
  await page.getByRole("button", { name: "Save public title" }).click();
  await expect(page.getByText("Outside Triage the public board shows it as A card is declined at checkout")).toBeVisible();
  await page.getByRole("radio", { name: "Done" }).click();
  for (const reason of ["Fixed", "Won't fix", "Duplicate"]) await expect(page.getByRole("radio", { name: reason })).toBeVisible();
  await page.getByRole("radio", { name: "In progress" }).click();
  await page.getByRole("button", { name: "Move ticket" }).click();
  await expect(page.getByText("Moved to In progress.")).toBeVisible();

  // The board's JSON is cacheable for 30 s (spec 10.1), so a new visitor checks the move.
  await visitor.close();
  const fresh = await browser.newContext({ storageState: { cookies: [], origins: [] } });
  const v2 = await fresh.newPage();
  await v2.goto(`/status/${slug}/tickets`);
  await expect(v2.getByText("A card is declined at checkout")).toBeVisible();
  await expect(v2.getByRole("heading", { name: "Triage" })).toHaveCount(0);
  await expect(v2.getByText(/^Reported \d{4}-\d{2}-\d{2}$/)).toBeVisible();
  await expect(v2.getByText(/^Updated \d{4}-\d{2}-\d{2}$/)).toBeVisible();
  await expect(v2.getByText("Budi")).toHaveCount(0);
  await expect(v2.getByText("4111")).toHaveCount(0);

  await v2.goto(`/status/${slug}/report`);
  await v2.getByLabel("Your ref ID").fill(refId);
  await v2.getByLabel("What went wrong").fill("Receipt shows the wrong currency");
  await v2.getByRole("button", { name: "Send report" }).click();
  await expect(v2.getByRole("alert")).toHaveText("The description needs 1 to 10 000 characters.");
  await v2.getByLabel("What happened").fill("IDR shown as USD");
  await v2.getByRole("button", { name: "Send report" }).click();
  await expect(v2.getByText("Thanks. Your report is ticket #2.")).toBeVisible();

  await v2.getByRole("link", { name: "My tickets" }).first().click();
  await v2.getByLabel("Ref ID").fill(refId);
  await v2.getByRole("button", { name: "Show my tickets" }).click();
  await expect(v2.getByText("Card of Budi declined")).toBeVisible();
  await expect(v2.getByText(/In progress/)).toBeVisible();
  await fresh.close();
});
