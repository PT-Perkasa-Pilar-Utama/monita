import { expect, test } from "@playwright/test";
import { apiHeartbeat, apiMonitor, apiProject } from "../lib/api";
import { BASE_URL } from "../lib/env";
import { authFile } from "../lib/roles";

test.use({ storageState: authFile("supervisor") });

/** A tech lead's project with a monitor and a heartbeat, made in the tech lead's own session. */
async function teamProject(browser: import("@playwright/test").Browser): Promise<{ slug: string; monitor: number; heartbeat: number }> {
  const tl = await browser.newContext({ storageState: authFile("tl"), baseURL: BASE_URL });
  const page = await tl.newPage();
  const slug = await apiProject(page, "Watched");
  const monitor = await apiMonitor(page, slug, { method: "POST", headers: "X-Key: s3cret", body: "{}" });
  const { id: heartbeat } = await apiHeartbeat(page, slug);
  await tl.close();
  return { slug, monitor, heartbeat };
}

test("a supervisor reads another team's project and its settings, with nothing to change", async ({ page, browser }) => {
  const { slug, monitor, heartbeat } = await teamProject(browser);

  await page.goto(`/projects/${slug}/monitors`);
  await expect(page.getByRole("link", { name: "API" })).toBeVisible();
  await expect(page.getByRole("link", { name: "New monitor" })).toHaveCount(0);

  await page.goto(`/projects/${slug}/monitors/${monitor}`);
  await expect(page.getByRole("heading", { name: "API" })).toBeVisible();
  await expect(page.getByRole("link", { name: "Edit" })).toHaveCount(0);

  await page.goto(`/projects/${slug}/heartbeats/${heartbeat}`);
  await expect(page.getByRole("heading", { name: "nightly-settlement" })).toBeVisible();
  await expect(page.locator("#ping-url-start")).toHaveCount(0);

  await page.goto(`/projects/${slug}/settings`);
  await expect(page.locator("#general-homepage")).toBeDisabled();
  await expect(page.getByRole("button", { name: "Save changes" })).toHaveCount(0);
  await expect(page.getByRole("heading", { name: "Danger zone" })).toHaveCount(0);

  const write = await page.request.post(`/api/ui/projects/${slug}/monitors`, { headers: { origin: BASE_URL }, data: { name: "Mine" } });
  expect(write.status()).toBe(403);
});

test("a supervisor opens the admin pages but cannot invite", async ({ page }) => {
  await page.goto("/");
  await page.getByRole("link", { name: "Admin", exact: true }).click();
  await expect(page.getByRole("heading", { name: "User management" })).toBeVisible();
  await expect(page.getByRole("button", { name: "Invite new user" })).toHaveCount(0);
  await page.getByRole("button", { name: /^Account:/ }).click();
  await expect(page.getByRole("menu").getByText("Supervisor", { exact: true })).toBeVisible();
});
