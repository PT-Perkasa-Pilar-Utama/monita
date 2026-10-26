import { expect, test } from "@playwright/test";
import { apiHeartbeat, apiMonitor, apiProject } from "../lib/api";
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

test("a TCP monitor needs a port, hides the HTTP fields, and Details tries it with nc", async ({ page }) => {
  const slug = await apiProject(page, "Ports");
  await page.goto(`/projects/${slug}/monitors/new`);
  await page.locator("#m-name").fill("Database");
  await page.getByRole("radio", { name: "TCP port" }).click();
  await expect(page.getByLabel("Host and port")).toBeVisible();
  await expect(page.locator("#m-method")).toHaveCount(0);
  await expect(page.getByText("Sign in with")).toHaveCount(0);
  await page.locator("#m-url").fill("127.0.0.1");
  await page.getByRole("button", { name: "Save monitor" }).click();
  await expect(page.getByText("Enter a host and a port, such as db.internal:5432")).toBeVisible();
  await page.locator("#m-url").fill("127.0.0.1:18080");
  await page.getByRole("button", { name: "Save monitor" }).click();
  await expect(page.getByRole("heading", { name: "Database" })).toBeVisible();
  await expect(page.getByText("TCP 127.0.0.1:18080")).toBeVisible();

  await page.getByRole("button", { name: "Details" }).click();
  await expect(page.locator("#monitor-curl")).toContainText("nc -vz -w 10 '127.0.0.1' 18080");
});

test("a ping monitor takes a host, and Details tries it with one ping", async ({ page }) => {
  const slug = await apiProject(page, "Pings");
  await page.goto(`/projects/${slug}/monitors/new`);
  await page.locator("#m-name").fill("Gateway");
  await page.getByRole("radio", { name: "Ping" }).click();
  await expect(page.locator('label[for="m-url"]')).toContainText("Host");
  await expect(page.locator("#m-method")).toHaveCount(0);
  await page.locator("#m-url").fill("127.0.0.1:80");
  await page.getByRole("button", { name: "Save monitor" }).click();
  await expect(page.getByText("Enter a host name or an address")).toBeVisible();
  await page.locator("#m-url").fill("127.0.0.1");
  await page.getByRole("button", { name: "Save monitor" }).click();
  await expect(page.getByText("Ping 127.0.0.1")).toBeVisible();

  await page.getByRole("button", { name: "Details" }).click();
  await expect(page.locator("#monitor-curl")).toContainText("ping -c 1 '127.0.0.1'");
});

test("an https monitor offers a certificate warning, and Details shows it", async ({ page }) => {
  const slug = await apiProject(page, "Certs");
  await page.goto(`/projects/${slug}/monitors/new`);
  await page.locator("#m-name").fill("Storefront");
  await page.locator("#m-url").fill("http://127.0.0.1:9/");
  await expect(page.getByLabel("Warn before the certificate expires")).toHaveCount(0);
  await page.locator("#m-url").fill("https://127.0.0.1:9/");
  await page.getByLabel("Warn before the certificate expires").click();
  await page.getByRole("option", { name: "14 days before" }).click();
  await page.getByRole("button", { name: "Save monitor" }).click();
  await expect(page.getByRole("heading", { name: "Storefront" })).toBeVisible();

  await page.getByRole("button", { name: "Details" }).click();
  await expect(page.getByText("14 days before, once per certificate")).toBeVisible();
  await expect(page.getByText("Not seen yet")).toBeVisible();
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

test("a fail ping shows its reason in the heartbeat's history, and the chart reads out where you point", async ({ page }) => {
  const slug = await apiProject(page, "Reasons");
  const { id, pingUrl } = await apiHeartbeat(page, slug);
  expect((await page.request.post(`${pingUrl}/fail`, { data: "disk full on /backup" })).status()).toBe(200);
  await page.goto(`/projects/${slug}/heartbeats/${id}`);
  await expect(page.getByRole("cell", { name: "disk full on /backup" })).toBeVisible();

  const chart = page.getByRole("img", { name: /^Recorded pings, last/ });
  await chart.hover();
  await expect(chart.locator("line[class*='stroke-foreground/40']")).toHaveCount(1);
});

test("a list sorts by a heading click, keeps the order in the URL, and a row click opens it", async ({ page }) => {
  const slug = await apiProject(page, "Sorting");
  await apiMonitor(page, slug, { name: "Alpha" });
  await apiMonitor(page, slug, { name: "Bravo" });
  await page.goto(`/projects/${slug}/monitors`);
  // The first column holds the selection box; the name is the row's link.
  const names = page.locator("tbody tr").getByRole("link");
  await page.getByRole("button", { name: "Monitor", exact: true }).click();
  await expect(page).toHaveURL(/[?&]sort=name/);
  const first = await names.first().textContent();
  await page.getByRole("button", { name: "Monitor", exact: true }).click();
  await expect(names.first()).not.toHaveText(first!);
  await page.locator("tbody tr").filter({ hasText: "Alpha" }).locator("td").nth(2).click();
  await expect(page.getByRole("heading", { name: "Alpha" })).toBeVisible();
});

test("a heartbeat page shows its four numbers: the streak, the next ping, the last ping and incidents", async ({ page }) => {
  const slug = await apiProject(page, "Stats");
  const { id, pingUrl } = await apiHeartbeat(page, slug);
  await page.goto(`/projects/${slug}/heartbeats/${id}`);
  for (const label of ["Created", "Next ping due", "Last ping", "Incidents, 30 days"]) await expect(page.getByText(label, { exact: true })).toBeVisible();
  await expect(page.getByText("the job has not called yet", { exact: true })).toBeVisible();
  expect((await page.request.get(pingUrl)).status()).toBe(200);
  await page.reload();
  for (const label of ["Up for", "Next ping due", "Last ping", "Incidents, 30 days"]) await expect(page.getByText(label, { exact: true })).toBeVisible();
  await expect(page.getByText("the job reported success", { exact: true })).toBeVisible();
});
