import { readFileSync } from "node:fs";
import { expect, type Page, test } from "@playwright/test";
import { apiHeartbeat, apiMonitor, apiProject } from "../lib/api";
import { BASE_URL } from "../lib/env";
import { authFile } from "../lib/roles";

test.use({ storageState: authFile("tl") });

test("General saves the homepage and the description", async ({ page }) => {
  const slug = await apiProject(page, "General");
  await page.goto(`/projects/${slug}/settings`);
  const section = page.locator("#project");
  await expect(section.locator("#general-homepage")).toHaveValue("https://app.example.com");
  await section.locator("#general-homepage").fill("https://reports.example.com");
  await section.locator("#general-description").fill("Monthly reports for finance.");
  await section.getByRole("button", { name: "Save changes" }).click();
  await expect(page.getByText("Project saved.")).toBeVisible();
  await page.reload();
  await expect(section.locator("#general-homepage")).toHaveValue("https://reports.example.com");
  await expect(section.locator("#general-description")).toHaveValue("Monthly reports for finance.");
});

test("@US-21 a status page tab turned off is gone from the public page and its JSON; the template downloads", async ({ page }) => {
  const slug = await apiProject(page, "Tabs");
  const { pingUrl } = await apiHeartbeat(page, slug);
  expect((await page.request.post(`${pingUrl}/fail`)).status()).toBe(200);
  const incidents = async (): Promise<number> =>
    ((await (await page.request.get(`/api/v1/public/status/${slug}`, { headers: { "cache-control": "no-cache" } })).json()) as { data: { incidents: unknown[] } }).data.incidents.length;
  expect(await incidents()).toBe(1);

  await page.goto(`/projects/${slug}/settings/status-page`);
  await page.locator("#tab-incidents").click();
  await page.locator("#tabs").getByRole("button", { name: "Save changes" }).click();
  await expect(page.getByText("Status page saved.")).toBeVisible();
  expect(await incidents()).toBe(0);
  await page.goto(`/status/${slug}/incidents`);
  await expect(page.getByRole("heading", { name: "Not found" })).toBeVisible();

  await page.goto(`/projects/${slug}/settings/status-page`);
  const [file] = await Promise.all([page.waitForEvent("download"), page.getByRole("button", { name: "Download template" }).click()]);
  expect(file.suggestedFilename()).toBe(`status-${slug}.html`);
  const html = readFileSync((await file.path())!, "utf8");
  expect(html).toContain(`const MONITA = "${BASE_URL}";`);
  expect(html).toContain(`const SLUG = "${slug}";`);
});

test("a secret in use cannot be deleted, and its value can be replaced", async ({ page }) => {
  const slug = await apiProject(page, "Secrets");
  await page.goto(`/projects/${slug}/settings/secrets`);
  await page.getByRole("button", { name: "New secret" }).click();
  await page.locator("#secret-name").fill("API_TOKEN");
  await page.locator("#secret-value").fill("first-value");
  await page.getByRole("button", { name: "Save secret" }).click();
  await expect(page.getByText("Secret API_TOKEN saved.")).toBeVisible();

  const secrets = await secretList(page, slug);
  await apiMonitor(page, slug, { auth_kind: "bearer", auth_secret_id: secrets[0]!.id });
  await page.reload();
  const row = page.getByRole("row", { name: /API_TOKEN/ });
  await expect(row.getByText("1 monitor")).toBeVisible();
  await expect(row.getByRole("button", { name: "Delete" })).toBeDisabled();
  await row.getByRole("button", { name: "Replace value" }).click();
  await page.locator("#secret-value").fill("second-value");
  await page.getByRole("button", { name: "Save new value" }).click();
  await expect(page.getByText("New value saved for API_TOKEN.")).toBeVisible();
});

async function secretList(page: Page, slug: string): Promise<{ id: number; name: string }[]> {
  const res = await page.request.get(`/api/ui/projects/${slug}/secrets`);
  return ((await res.json()) as { data: { id: number; name: string }[] }).data;
}

test("the allowlist saves, and with both owner cards off the page says no one hears", async ({ page }) => {
  const slug = await apiProject(page, "Allowlist");
  await page.goto(`/projects/${slug}/settings/allowlist`);
  await page.locator("#cidrs").fill("10.0.0.0/8");
  await page.getByRole("button", { name: "Save changes" }).click();
  await expect(page.getByText("Allowlist saved.")).toBeVisible();

  await page.goto(`/projects/${slug}/settings/alerts`);
  const owner = page.getByRole("list", { name: "Alert cards" }).getByRole("listitem");
  await owner.nth(0).getByRole("switch").click();
  await expect(page.getByText("No card listens to critical alerts.", { exact: false })).toBeVisible();
  await owner.nth(1).getByRole("switch").click();
  await expect(page.getByText("No card listens to warning alerts.", { exact: false })).toBeVisible();
  await page.reload();
  await expect(owner.nth(0).getByRole("switch")).not.toBeChecked();
  await expect(page.getByText("No card listens to critical alerts.", { exact: false })).toBeVisible();
});

test("ticket alerts keep the column-move box once saved", async ({ page }) => {
  const slug = await apiProject(page, "Ticket alerts");
  await page.goto(`/projects/${slug}/settings/alerts`);
  const tickets = page.locator("#tickets");
  await tickets.locator("#ticket-webhook-on").click();
  await tickets.locator("#ticket-url").fill("https://hooks.example.com/tickets");
  await tickets.locator("#ticket-webhook-moves").click();
  await tickets.getByRole("button", { name: "Save changes" }).click();
  await expect(page.getByText("Ticket alerts saved.")).toBeVisible();
  await page.reload();
  await expect(tickets.locator("#ticket-webhook-moves")).toBeChecked();
});
