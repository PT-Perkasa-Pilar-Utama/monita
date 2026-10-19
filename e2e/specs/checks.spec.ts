import { expect, test } from "@playwright/test";
import { createProject } from "../lib/project";
import { authFile } from "../lib/roles";

test.use({ storageState: authFile("tl") });

async function newProject(page: import("@playwright/test").Page): Promise<string> {
  const slug = await createProject(page, "Checks");
  return slug;
}

test("@US-09 TL adds a monitor with defaults and sees it waiting for its first check", async ({ page }) => {
  const slug = await newProject(page);
  await page.getByRole("link", { name: "New monitor" }).click();
  await expect(page.getByLabel("Check every")).toHaveText("1 minute");
  await expect(page.getByLabel("Timeout in seconds")).toHaveValue("10");
  await page.getByLabel("Name").fill("Monita health");
  await page.getByLabel("URL").fill("http://127.0.0.1:18080/healthz");
  await page.getByRole("button", { name: "Save monitor" }).click();
  await expect(page.getByRole("heading", { name: "Monita health" })).toBeVisible();
  await expect(page.locator("#state")).toHaveText("No data yet");

  await page.goto(`/projects/${slug}/monitors/new`);
  await page.getByLabel("Name").fill("Broken");
  await page.getByLabel("URL").fill("ftp://nope");
  await page.getByRole("button", { name: "Save monitor" }).click();
  await expect(page.getByText("The URL must start with http:// or https://.")).toBeVisible();
});

test("@US-12 @US-13 @US-14 a job pings its heartbeat URL and the state follows", async ({ page }) => {
  await newProject(page);
  await page.getByRole("link", { name: "Heartbeats" }).click();
  await page.getByRole("link", { name: "New heartbeat" }).click();
  await page.getByLabel("Name").fill("nightly-settlement");
  await page.getByRole("button", { name: "Save heartbeat" }).click();
  await expect(page.locator("#state")).toHaveText("No data yet");
  const pingUrl = (await page.locator("#ping-url").textContent())!.trim();

  expect((await page.request.get(pingUrl)).status()).toBe(200);
  await page.reload();
  await expect(page.locator("#state")).toHaveText("Up");

  await page.waitForTimeout(1_100);
  expect((await page.request.post(`${pingUrl}/fail`)).status()).toBe(200);
  await page.reload();
  await expect(page.locator("#state")).toHaveText("Down");
});
