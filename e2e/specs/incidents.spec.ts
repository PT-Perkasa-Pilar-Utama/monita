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

test("@US-15 TL adds an email card, fixes a bad address, and sends a test", async ({ page }) => {
  const { slug } = await projectWithHeartbeat(page);
  await page.goto(`/projects/${slug}/settings/alerts`);
  const cards = page.getByRole("list", { name: "Alert cards" });
  await expect(cards.getByRole("listitem")).toHaveCount(2);
  await expect(cards.getByRole("listitem").first()).toContainText("the project owner");
  await expect(cards.getByRole("listitem").first().getByRole("button", { name: "Edit" })).toHaveCount(0);

  await page.getByRole("button", { name: "Add alert card" }).click();
  await page.locator("#card-name").fill("On call");
  await page.locator("#card-emails").fill("oncall@monita.test\nnot-an-email");
  await page.getByRole("button", { name: "Add card" }).click();
  await expect(page.getByText("Not a valid email address: not-an-email")).toBeVisible();
  await page.locator("#card-emails").fill("oncall@monita.test");
  await page.getByRole("button", { name: "Add card" }).click();
  const card = cards.getByRole("listitem", { name: "On call" });
  await expect(card).toContainText("oncall@monita.test");
  await card.getByRole("button", { name: "Send a test" }).click();
  await expect(page.getByText("Test sent to 1 recipient.")).toBeVisible();
});

test("a PagerDuty card needs a routing key and no URL", async ({ page }) => {
  const { slug } = await projectWithHeartbeat(page);
  await page.goto(`/projects/${slug}/settings/alerts`);
  await page.getByRole("button", { name: "Add alert card" }).click();
  await page.locator("#card-kind").click();
  await page.getByRole("option", { name: "PagerDuty" }).click();
  await expect(page.locator("#card-url")).toHaveCount(0);
  await expect(page.locator("#card-name")).toHaveValue("PagerDuty");
  await page.getByRole("button", { name: "Add card" }).click();
  await expect(page.getByText("Enter the routing key of the PagerDuty integration.")).toBeVisible();
  await page.locator("#card-key").fill("R0utingKey1");
  await page.getByRole("button", { name: "Add card" }).click();
  const card = page.getByRole("list", { name: "Alert cards" }).getByRole("listitem", { name: "PagerDuty" });
  await expect(card).toContainText("PagerDuty Events API");
  await page.reload();
  await card.getByRole("button", { name: "Edit" }).click();
  await expect(page.locator("#card-kind")).toHaveText("PagerDuty");
  await expect(page.locator("#card-key")).toHaveValue("R0utingKey1");
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

test("a maintenance window holds back the incident until the tech lead ends it", async ({ page }) => {
  const { slug, pingUrl } = await projectWithHeartbeat(page);
  await page.goto(`/projects/${slug}/incidents`);
  await page.getByRole("link", { name: "Maintenance" }).click();
  await page.getByRole("link", { name: "Plan maintenance" }).click();
  await page.getByRole("button", { name: "Plan maintenance" }).click();
  await expect(page.getByText("Enter a title of up to 120 characters.")).toBeVisible();
  await page.getByLabel("Title").fill("Bank API upgrade");
  await page.getByLabel("nightly-settlement").check();
  await page.getByLabel("Message on the status page").fill("Settlement runs late tonight.");
  await page.getByRole("button", { name: "Plan maintenance" }).click();
  await expect(page.getByText("Maintenance planned.")).toBeVisible();
  const row = page.getByRole("row", { name: /Bank API upgrade/ });
  await expect(row.getByText("Active")).toBeVisible();

  expect((await page.request.post(`${pingUrl}/fail`, { data: "planned" })).status()).toBe(200);
  await page.goto(`/projects/${slug}/heartbeats`);
  await expect(page.getByText("Maintenance").first()).toBeVisible();
  await page.goto(`/projects/${slug}/incidents`);
  await expect(page.getByText("Every system is up")).toBeVisible();

  await page.goto(`/projects/${slug}/incidents/maintenance`);
  await row.getByRole("button", { name: "End now" }).click();
  await page.getByRole("alertdialog").getByRole("button", { name: "End now" }).click();
  await expect(row.getByText("Ended")).toBeVisible();
  // `/fail` takes one ping a second; the first may still hold this second.
  await expect.poll(async () => (await page.request.post(`${pingUrl}/fail`, { data: "real trouble" })).status(), { intervals: [1100] }).toBe(200);
  await page.goto(`/projects/${slug}/incidents`);
  await expect(page.getByRole("link", { name: "nightly-settlement reported a failure" })).toBeVisible();
});

test("a tech lead resolves an incident Monita opened, and it closes at once", async ({ page }) => {
  const { slug, pingUrl } = await projectWithHeartbeat(page);
  expect((await page.request.post(`${pingUrl}/fail`, { data: "disk full" })).status()).toBe(200);
  await page.goto(`/projects/${slug}/incidents`);
  await page.getByRole("link", { name: "nightly-settlement reported a failure" }).click();
  await expect(page.locator("#state")).toContainText("Open");
  await page.getByRole("button", { name: "Post update" }).click();
  const dialog = page.getByRole("dialog");
  await dialog.getByRole("radio", { name: "Resolved" }).click();
  await expect(dialog.getByText("Closes it now and tells the alert recipients.", { exact: false })).toBeVisible();
  await dialog.getByLabel("Message").fill("The disk was cleaned up by hand.");
  await dialog.getByRole("button", { name: "Post update" }).click();
  await expect(page.locator("#state")).toContainText("Resolved");
  await expect(page.locator('[data-update="tech-lead"]')).toHaveText("The disk was cleaned up by hand.");
});
