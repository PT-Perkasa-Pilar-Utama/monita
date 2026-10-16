import { expect, type Page, test } from "@playwright/test";
import { authFile } from "../lib/roles";

test.use({ storageState: authFile("tl") });

async function projectWithHeartbeat(page: Page): Promise<{ slug: string; pingUrl: string }> {
  const slug = `i-${Date.now()}-${Math.floor(Math.random() * 1000)}`;
  await page.goto("/");
  await page.getByLabel("Name").fill("Incidents");
  await page.getByLabel("Slug").fill(slug);
  await page.getByRole("button", { name: "Create project" }).click();
  await page.goto(`/projects/${slug}/heartbeats/new`);
  await page.getByLabel("Name").fill("nightly-settlement");
  await page.getByRole("button", { name: "Save heartbeat" }).click();
  const pingUrl = (await page.locator("#ping-url").textContent())!.trim();
  return { slug, pingUrl };
}

test("@US-15 TL sets who gets critical alerts and sends a test", async ({ page }) => {
  const { slug } = await projectWithHeartbeat(page);
  await page.goto(`/projects/${slug}/alerts`);
  await expect(page.locator("#use-critical")).toBeChecked();
  await expect(page.locator("#use-warning")).not.toBeChecked();
  await page.locator("#emails-critical").fill("oncall@monita.test\nnot-an-email");
  await page.getByRole("button", { name: "Save critical alerts" }).click();
  await expect(page.getByRole("alert")).toHaveText("Not a valid email address: not-an-email");
  await page.locator("#emails-critical").fill("oncall@monita.test");
  await page.getByRole("button", { name: "Save critical alerts" }).click();
  await page.locator("#critical").getByRole("button", { name: "Send a test" }).click();
  await expect(page.getByRole("status")).toHaveText("Test queued for 1 recipient. It arrives within a minute.");
});

test("@US-14 @US-17 @US-18 @US-19 a failing job opens an incident, the TL updates it, the next ping closes it", async ({ page }) => {
  const { slug, pingUrl } = await projectWithHeartbeat(page);
  expect((await page.request.post(`${pingUrl}/fail`, { data: "settle.sh: bank API timed out" })).status()).toBe(200);

  await page.goto(`/projects/${slug}/incidents`);
  await page.getByRole("link", { name: "nightly-settlement reported a failure" }).click();
  await expect(page.getByText("settle.sh: bank API timed out").first()).toBeVisible();

  await page.getByRole("button", { name: "Post update" }).click();
  await expect(page.getByRole("alert")).toHaveText("Write a message.");
  await page.getByLabel("Label").selectOption("identified");
  await page.getByLabel("Message").fill("The bank rotated its certificate. Updating the trust store.");
  await page.getByRole("button", { name: "Post update" }).click();
  await expect(page.locator(".comment")).toHaveText("The bank rotated its certificate. Updating the trust store.");

  await page.waitForTimeout(1_100);
  expect((await page.request.get(pingUrl)).status()).toBe(200);
  await page.reload();
  await expect(page.getByText(/Recovered after/)).toBeVisible();
  await expect(page.locator(".page-head .cap")).toContainText("resolved");
});

test("@US-20 TL reports an incident by hand and resolves it", async ({ page }) => {
  const { slug } = await projectWithHeartbeat(page);
  await page.goto(`/projects/${slug}/incidents/new`);
  await page.getByLabel("Title").fill("Slow settlement reports");
  await page.getByRole("button", { name: "Open incident" }).click();
  await expect(page.getByRole("alert")).toHaveText("Pick at least one monitor or heartbeat.");
  await page.getByLabel("nightly-settlement").check();
  await page.getByLabel("First update").fill("Reports take 20 minutes instead of 2.");
  await page.getByRole("button", { name: "Open incident" }).click();
  await expect(page.getByRole("heading", { name: "Slow settlement reports" })).toBeVisible();

  await page.goto(`/projects/${slug}/heartbeats`);
  await expect(page.locator("tbody .state")).toHaveText("Issue");

  await page.goto(`/projects/${slug}/incidents`);
  await page.getByRole("link", { name: "Slow settlement reports" }).click();
  await page.getByLabel("Label").selectOption("resolved");
  await page.getByLabel("Message").fill("Index added. Reports take 90 seconds.");
  await page.getByRole("button", { name: "Post update" }).click();
  await expect(page.locator(".page-head .cap")).toContainText("resolved");
});
