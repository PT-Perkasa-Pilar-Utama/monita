import { expect, type Page, test } from "@playwright/test";
import { createProject } from "../lib/project";
import { authFile } from "../lib/roles";

test.use({ storageState: authFile("tl") });

async function projectWithHeartbeat(page: Page): Promise<{ slug: string; pingUrl: string }> {
  const slug = await createProject(page, "Incidents");
  await page.goto(`/projects/${slug}/heartbeats/new`);
  await page.getByLabel("Name").fill("nightly-settlement");
  await page.getByRole("button", { name: "Save heartbeat" }).click();
  const pingUrl = (await page.locator("#ping-url-start").textContent())!.trim();
  return { slug, pingUrl };
}

test("@US-15 TL sets who gets critical alerts and sends a test", async ({ page }) => {
  const { slug } = await projectWithHeartbeat(page);
  await page.goto(`/projects/${slug}/settings/alerts`);
  const critical = page.locator("#critical");
  await expect(page.locator("#use-critical")).toBeChecked();
  await expect(page.locator("#use-warning")).not.toBeChecked();
  await page.locator("#emails-critical").fill("oncall@monita.test\nnot-an-email");
  await critical.getByRole("button", { name: "Save changes" }).click();
  await expect(critical.getByRole("alert")).toHaveText("Not a valid email address: not-an-email");
  await page.locator("#emails-critical").fill("oncall@monita.test");
  await critical.getByRole("button", { name: "Save changes" }).click();
  await critical.getByRole("button", { name: "Send a test" }).click();
  await expect(critical.getByText("Test sent to 1 recipient.")).toBeVisible();
});

test("@US-14 @US-17 @US-18 @US-19 a failing job opens an incident, the TL updates it, the next ping closes it", async ({ page }) => {
  const { slug, pingUrl } = await projectWithHeartbeat(page);
  expect((await page.request.post(`${pingUrl}/fail`, { data: "settle.sh: bank API timed out" })).status()).toBe(200);

  await page.goto(`/projects/${slug}/incidents`);
  await page.getByRole("link", { name: "nightly-settlement reported a failure" }).click();
  await expect(page.getByText("settle.sh: bank API timed out").first()).toBeVisible();

  await page.getByRole("button", { name: "Post update" }).click();
  const dialog = page.getByRole("dialog");
  // Each label starts from its template until the TL writes their own words.
  await dialog.getByRole("radio", { name: "Monitoring" }).click();
  await expect(dialog.getByLabel("Message")).toHaveValue("A fix is out. We are watching to make sure it holds.");
  await dialog.getByLabel("Message").fill("Our own words.");
  await dialog.getByRole("radio", { name: "Identified" }).click();
  await expect(dialog.getByLabel("Message")).toHaveValue("Our own words.");
  await dialog.getByLabel("Message").fill("");
  await dialog.getByRole("button", { name: "Post update" }).click();
  await expect(dialog.getByRole("alert")).toHaveText("Write a message.");
  await dialog.getByRole("radio", { name: "Identified" }).click();
  await dialog.getByLabel("Message").fill("The bank rotated its certificate. Updating the trust store.");
  await dialog.getByRole("button", { name: "Post update" }).click();
  await expect(page.locator('[data-update="tech-lead"]')).toHaveText("The bank rotated its certificate. Updating the trust store.");

  await page.waitForTimeout(1_100);
  expect((await page.request.get(pingUrl)).status()).toBe(200);
  await page.reload();
  await expect(page.getByText(/Recovered after/)).toBeVisible();
  await expect(page.locator("#state")).toContainText("Resolved");
  // AC-18.02: the facts name the start, the end and how long it lasted.
  for (const fact of ["Started", "Ended", "Lasted"]) await expect(page.locator("dt", { hasText: fact })).toBeVisible();
  // admin.spec.ts may hold another offset for a moment on a shared server.
  await expect(page.locator("dt", { hasText: "Ended" }).locator("+ dd")).toHaveText(/UTC([+-][\d:]+)?$/);
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
  await expect(page.locator("tbody").getByText("Issue", { exact: true })).toBeVisible();

  await page.goto(`/projects/${slug}/incidents`);
  await page.getByRole("link", { name: "Slow settlement reports" }).click();
  await page.getByRole("button", { name: "Post update" }).click();
  const dialog = page.getByRole("dialog");
  // Each label starts from its template until the TL writes their own words.
  await dialog.getByRole("radio", { name: "Monitoring" }).click();
  await expect(dialog.getByLabel("Message")).toHaveValue("A fix is out. We are watching to make sure it holds.");
  await dialog.getByLabel("Message").fill("Our own words.");
  await dialog.getByRole("radio", { name: "Identified" }).click();
  await expect(dialog.getByLabel("Message")).toHaveValue("Our own words.");
  await dialog.getByRole("radio", { name: "Resolved" }).click();
  await dialog.getByLabel("Message").fill("Index added. Reports take 90 seconds.");
  await dialog.getByRole("button", { name: "Post update" }).click();
  await expect(page.locator("#state")).toContainText("Resolved");
});
