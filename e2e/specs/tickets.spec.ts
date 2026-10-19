import { expect, test } from "@playwright/test";
import { apiProject } from "../lib/api";
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

  await v.goto(`/status/${slug}/issues`);
  await expect(v.getByText("Card of Budi declined")).toHaveCount(0);

  await page.goto(`/projects/${slug}/tickets`);
  for (const column of ["Triage", "Backlog", "In progress", "Done"]) await expect(page.getByRole("heading", { name: column, level: 2 })).toBeVisible();
  await page.getByRole("link", { name: /Card of Budi declined/ }).click();
  await expect(page.getByText("Card 4111 declined at the last step")).toBeVisible();
  await page.getByRole("button", { name: "Edit the public title" }).click();
  await page.getByLabel("Public title").fill("A card is declined at checkout");
  await page.getByRole("button", { name: "Save public title" }).click();
  await expect(page.getByText("Public title saved.")).toBeVisible();
  await page.getByRole("combobox", { name: "Column" }).click();
  await page.getByRole("option", { name: "Done" }).click();
  await expect(page.getByLabel("Close reason")).toBeVisible();
  await page.getByRole("combobox", { name: "Column" }).click();
  await page.getByRole("option", { name: "In progress" }).click();
  await expect(page.getByText("Moved to In progress.")).toBeVisible();

  // The board's JSON is cacheable for 30 s (spec 10.1), so a new visitor checks the move.
  await visitor.close();
  const fresh = await browser.newContext({ storageState: { cookies: [], origins: [] } });
  const v2 = await fresh.newPage();
  await v2.goto(`/status/${slug}/issues`);
  await expect(v2.getByText("A card is declined at checkout")).toBeVisible();
  await expect(v2.getByRole("heading", { name: "Triage" })).toHaveCount(0);
  await expect(v2.getByText(/^Reported \d{4}-\d{2}-\d{2}$/)).toBeVisible();
  await expect(v2.getByText("Budi")).toHaveCount(0);
  await expect(v2.getByText("4111")).toHaveCount(0);

  await v2.goto(`/status/${slug}/ticket`);
  await v2.locator("#m-ref").fill(refId);
  await v2.getByRole("button", { name: "Show my tickets" }).click();
  await expect(v2.getByText("Card of Budi declined")).toBeVisible();
  await v2.locator("#r-ref").fill(refId);
  await v2.getByLabel("What went wrong").fill("Receipt shows the wrong currency");
  await v2.getByRole("button", { name: "Send report" }).click();
  await expect(v2.getByRole("alert")).toHaveText("The description needs 1 to 10 000 characters.");
  await v2.getByLabel("What happened").fill("IDR shown as USD");
  await v2.getByRole("button", { name: "Send report" }).click();
  await expect(v2.getByText("Thanks. Your report is ticket #2.")).toBeVisible();
  await expect(v2.getByText("Receipt shows the wrong currency")).toBeVisible();
  await expect(v2.getByText(/In progress/)).toBeVisible();
  await fresh.close();
});

test("the team files a ticket, issues a ref ID, drags it across to Done, and Done keeps its own list", async ({ page }) => {
  const slug = await apiProject(page, "Board");
  await page.goto(`/projects/${slug}/tickets`);

  await page.getByRole("button", { name: "New ticket" }).click();
  await page.locator("#ticket-title").fill("Export button stays grey");
  await page.locator("#ticket-description").fill("Seen on the March report.");
  await page.getByRole("button", { name: "File ticket" }).click();
  await expect(page.getByText("Ticket #1 filed.")).toBeVisible();

  await page.getByRole("button", { name: "Issue a ref ID" }).click();
  await page.getByRole("dialog").getByRole("button", { name: "Issue ref ID" }).click();
  await expect(page.locator("#issued-ref-id")).toHaveText(/^rf_[a-z2-7]{26}$/);
  await page.keyboard.press("Escape");

  // A team-filed ticket starts in Backlog.
  const column = (stage: string) => page.locator(`section[aria-labelledby='col-${stage}']`);
  const card = page.getByRole("link", { name: /Export button stays grey/ });
  await expect(column("backlog").getByText("Export button stays grey")).toBeVisible();
  await card.dragTo(column("in_progress"));
  await expect(page.getByText("Moved #1 to In progress.")).toBeVisible();
  await card.dragTo(column("done"));
  const closing = page.getByRole("dialog", { name: "Move #1 to Done" });
  await expect(closing.getByRole("button", { name: "Move to Done" })).toBeDisabled();
  await closing.getByRole("radio", { name: "Fixed" }).click();
  await closing.getByRole("button", { name: "Move to Done" }).click();
  await expect(column("done").getByText("Export button stays grey")).toBeVisible();

  // Done keeps its own paged list.
  await page.goto(`/projects/${slug}/tickets/done`);
  await expect(page.getByRole("heading", { name: "Done tickets" })).toBeVisible();
  await expect(page.getByText("Export button stays grey")).toBeVisible();
});
