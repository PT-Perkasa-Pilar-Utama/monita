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
  await page.locator("#card-emails").fill("oncall@monita.test\nnot-an-email");
  await page.getByRole("button", { name: "Add card" }).click();
  await expect(page.getByText("Not a valid email address: not-an-email")).toBeVisible();
  await page.locator("#card-emails").fill("oncall@monita.test");
  await page.getByRole("button", { name: "Add card" }).click();
  // The form asks for no name: an email card is called Email.
  const card = cards.getByRole("listitem", { name: "Email", exact: true });
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
  await expect(dialog.getByLabel("Message")).toHaveValue("A fix is out and we are watching to make sure it holds.");
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
  // Team pages show the viewer's own clock, with no zone name.
  await expect(page.locator("dt", { hasText: "Ended" }).locator("+ dd")).toHaveText(/^\d{4}-\d{2}-\d{2} \d{2}:\d{2}$/);
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
  await expect(dialog.getByLabel("Message")).toHaveValue("A fix is out and we are watching to make sure it holds.");
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
  await expect(row.getByRole("link", { name: "nightly-settlement" })).toBeVisible();

  // An active window edits its title, message and end; its start and items stay (Q50).
  await row.getByRole("link", { name: "Edit" }).click();
  const form = page.getByRole("dialog");
  await expect(form.getByLabel("Starts")).toBeDisabled();
  await expect(form.getByText("An active window keeps its items", { exact: false })).toBeVisible();
  await form.getByLabel("Title").fill("Bank API upgrade, longer");
  await form.getByRole("button", { name: "Save window" }).click();
  await expect(page.getByText("Window saved.")).toBeVisible();
  await expect(row.getByText("Bank API upgrade, longer")).toBeVisible();

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
  await expect(dialog.getByText("Closes it now and tells the alert recipients", { exact: false })).toBeVisible();
  await dialog.getByLabel("Message").fill("The disk was cleaned up by hand.");
  await dialog.getByRole("button", { name: "Post update" }).click();
  await expect(page.locator("#state")).toContainText("Resolved");
  await expect(page.locator('[data-update="tech-lead"]')).toHaveText("The disk was cleaned up by hand.");
});

test("a tech lead pauses a heartbeat, plans maintenance from its row menu, and resumes it", async ({ page }) => {
  const { slug, pingUrl } = await projectWithHeartbeat(page);
  await page.goto(`/projects/${slug}/heartbeats`);
  await page.getByRole("link", { name: "nightly-settlement" }).click();
  await page.getByRole("button", { name: "Pause" }).click();
  await page.getByRole("alertdialog").getByRole("button", { name: "Pause" }).click();
  await expect(page.locator("#state")).toHaveText("Paused");
  await expect(page.getByRole("status").getByText(/^Paused .+, so Monita keeps its pings/)).toBeVisible();
  expect((await page.request.post(`${pingUrl}/fail`, { data: "while paused" })).status()).toBe(200);
  await page.goto(`/projects/${slug}/incidents`);
  await expect(page.getByText("Every system is up")).toBeVisible();

  // Plan maintenance lives in the row menu of the list.
  await page.goto(`/projects/${slug}/heartbeats`);
  await page.getByRole("button", { name: "Actions for nightly-settlement" }).click();
  await page.getByRole("menuitem", { name: "Plan maintenance" }).click();
  await expect(page.getByRole("dialog").getByLabel("nightly-settlement")).toBeChecked();
  await page.goBack();
  await page.getByRole("link", { name: "nightly-settlement" }).click();
  await page.getByRole("button", { name: "Resume" }).click();
  await expect(page.locator("#state")).not.toHaveText("Paused");
});

test("a tech lead marks an incident as a false alarm, the cross-project list tags it, and it counts again", async ({ page }) => {
  const { slug, pingUrl } = await projectWithHeartbeat(page);
  expect((await page.request.post(`${pingUrl}/fail`, { data: "disk full" })).status()).toBe(200);
  await page.goto(`/projects/${slug}/incidents`);
  await page.getByRole("link", { name: /nightly-settlement/ }).first().click();
  await page.getByRole("button", { name: "Reclassify" }).click();
  await page.getByRole("menuitem", { name: "False alarm" }).click();
  await page.getByRole("alertdialog").getByRole("button").last().click();
  await expect(page.getByText("Marked as false alarm.")).toBeVisible();
  const id = page.url().split("/").pop();
  await page.goto("/incidents?state=all");
  await expect(page.locator("tbody tr").filter({ hasText: `INC-${id}` }).getByText("False alarm")).toBeVisible();
  await page.goBack();
  await page.getByRole("button", { name: "Reclassify" }).click();
  await page.getByRole("menuitem", { name: "Count it again" }).click();
  await page.getByRole("alertdialog").getByRole("button").last().click();
  await expect(page.getByText("The incident counts again.")).toBeVisible();
});

test("the project and an incident download their reports as PDF, and a range that is too long is refused", async ({ page }) => {
  const { slug, pingUrl } = await projectWithHeartbeat(page);
  expect((await page.request.post(`${pingUrl}/fail`)).status()).toBe(200);
  await page.goto(`/projects/${slug}`);

  const menu = page.getByRole("button", { name: "Download report" });
  const monthly = page.waitForEvent("download");
  await menu.click();
  await page.getByRole("menuitem", { name: "Last month" }).click();
  expect((await monthly).suggestedFilename()).toMatch(new RegExp(`^${slug}-\\d{4}-\\d{2}-01-to-\\d{4}-\\d{2}-\\d{2}\\.pdf$`));

  await menu.click();
  await page.getByRole("menuitem", { name: "Pick the days" }).click();
  // The browser holds a last day before the first; the server refuses a range that is too long.
  await page.getByLabel("First day").fill("2025-01-01");
  await page.getByLabel("Last day").fill("2026-10-31");
  await page.getByRole("dialog").getByRole("button", { name: "Download" }).click();
  await expect(page.getByRole("dialog")).toContainText("A report covers 366 days at most.");
  await page.getByLabel("First day").fill("2026-10-05");
  const picked = page.waitForEvent("download");
  await page.getByRole("dialog").getByRole("button", { name: "Download" }).click();
  expect((await picked).suggestedFilename()).toBe(`${slug}-2026-10-05-to-2026-10-31.pdf`);
  await expect(page.getByRole("dialog")).toHaveCount(0);

  await page.goto(`/projects/${slug}/incidents`);
  await page.getByRole("link", { name: "nightly-settlement reported a failure" }).click();
  const one = page.waitForEvent("download");
  await page.getByRole("button", { name: "Download report" }).click();
  expect((await one).suggestedFilename()).toMatch(new RegExp(`^${slug}-inc-\\d+\\.pdf$`));
});

test("a resolved incident takes a post-mortem that is saved as a draft and then published", async ({ page }) => {
  const { slug, pingUrl } = await projectWithHeartbeat(page);
  expect((await page.request.post(`${pingUrl}/fail`)).status()).toBe(200);
  await page.goto(`/projects/${slug}/incidents`);
  await page.getByRole("link", { name: "nightly-settlement reported a failure" }).click();
  await expect(page.getByRole("heading", { name: "Post-mortem" })).toHaveCount(0);
  expect((await page.request.get(pingUrl)).status()).toBe(200);
  await page.reload();

  // The template states the facts, and its prompts stop a publish.
  await page.getByRole("button", { name: "Write post-mortem" }).click();
  await page.getByRole("button", { name: "Fill in a template" }).click();
  await expect(page.getByLabel("Summary")).toHaveValue(/^INC-\d+: nightly-settlement reported a failure\./);
  await expect(page.getByLabel("Impact")).toHaveValue(/\[Who was affected/);
  await expect(page.getByRole("button", { name: "Fill in a template" })).toBeDisabled();
  await page.getByRole("button", { name: "Publish" }).click();
  await expect(page.getByText("Replace the text in square brackets before you publish.").first()).toBeVisible();
  await expect(page.getByRole("alertdialog")).toHaveCount(0);
  await page.keyboard.press("Escape");

  await page.getByRole("button", { name: "Write post-mortem" }).click();
  await page.getByLabel("Summary").fill("The settlement job crashed on a bad row.");
  await page.getByRole("button", { name: "Save draft" }).click();
  await expect(page.getByText("Draft saved.")).toBeVisible();
  await expect(page.locator("#postmortem").locator("..")).toContainText("Draft");

  await page.getByRole("button", { name: "Continue the draft" }).click();
  await page.getByRole("button", { name: "Publish" }).click();
  await expect(page.getByRole("alertdialog")).toContainText("no email goes out");
  await page.getByRole("alertdialog").getByRole("button", { name: "Publish" }).click();
  await expect(page.getByText("Write the root cause before you publish.")).toBeVisible();
  await page.getByLabel("Root cause").fill("A row with an empty amount.");
  await page.getByLabel("Resolution").fill("The job skips such rows and reports them.");
  await page.getByRole("button", { name: "Publish" }).click();
  await page.getByRole("alertdialog").getByRole("button", { name: "Publish" }).click();
  await expect(page.getByText("Post-mortem published.")).toBeVisible();
  const panel = page.locator("section", { has: page.locator("#postmortem") });
  await expect(panel).toContainText("Published");
  await expect(panel).toContainText("A row with an empty amount.");
});
