import { expect, test } from "@playwright/test";
import { apiHeartbeat, apiProject } from "../lib/api";
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
  await expect(page.getByLabel("Timeout")).toHaveValue("10");
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
  const pingUrl = (await page.locator("#ping-url-start").textContent())!.trim();

  expect((await page.request.get(pingUrl)).status()).toBe(200);
  await page.reload();
  await expect(page.locator("#state")).toHaveText("Up");

  await page.waitForTimeout(1_100);
  expect((await page.request.post(`${pingUrl}/fail`)).status()).toBe(200);
  await page.reload();
  await expect(page.locator("#state")).toHaveText("Down");
});

test("a POST monitor sends its header and body, counts down after 1 failure, and Details repeats it as curl", async ({ page }) => {
  const slug = await apiProject(page, "Request");
  await page.goto(`/projects/${slug}/monitors/new`);
  await page.locator("#m-name").fill("Orders API");
  await page.locator("#m-method").click();
  await page.getByRole("option", { name: "POST" }).click();
  await page.locator("#m-url").fill("http://127.0.0.1:9/orders");
  await page.locator("#m-body").fill('{"ping": true}');
  await page.getByRole("button", { name: "Add header" }).click();
  await page.getByLabel("Header 1 name").fill("X-Env");
  await page.getByLabel("Header 1 value").fill("production");
  await page.locator("#m-fails").fill("1");
  await page.getByRole("button", { name: "Save monitor" }).click();
  await expect(page.getByRole("heading", { name: "Orders API" })).toBeVisible();

  await page.getByRole("button", { name: "Details" }).click();
  const curl = page.locator("#monitor-curl");
  await expect(curl).toContainText("-X POST");
  await expect(curl).toContainText("X-Env: production");
  await expect(curl).toContainText('{"ping": true}');
  await expect(page.getByRole("dialog").getByText("1 failed check")).toBeVisible();
});

test("heartbeat Details gives the call in Shell, Node.js and Python, and remembers the pick", async ({ page }) => {
  const slug = await apiProject(page, "Snippets");
  const { id, pingUrl } = await apiHeartbeat(page, slug);
  await page.goto(`/projects/${slug}/heartbeats/${id}`);
  await page.getByRole("button", { name: "Details" }).click();
  const first = page.locator("#ping-snippet-0");
  await expect(first).toContainText("curl");
  await expect(first).toContainText(pingUrl);
  await page.getByRole("radio", { name: "Python" }).click();
  await expect(first).toContainText("urllib");
  await page.reload();
  await page.getByRole("button", { name: "Details" }).click();
  await expect(page.getByRole("radio", { name: "Python" })).toBeChecked();
});
