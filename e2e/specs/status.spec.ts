import { expect, test } from "@playwright/test";
import { apiHeartbeat, apiMonitor, apiProject, apiSend } from "../lib/api";
import { createProject } from "../lib/project";
import { authFile } from "../lib/roles";

test.describe("as the TL", () => {
  test.use({ storageState: authFile("tl") });

  test("@US-21 @US-22 a visitor sees public items only, with 90 day bars, and the same data as JSON", async ({ page, browser }) => {
    const slug = await createProject(page, "Storefront");

    await page.goto(`/projects/${slug}/monitors/new`);
    await page.getByLabel("Name").fill("Checkout");
    await page.getByLabel("URL").fill("http://127.0.0.1:18080/healthz");
    await page.getByLabel(/^Show this (monitor|heartbeat) and its incidents on the public status page$/).check();
    await page.getByRole("button", { name: "Save monitor" }).click();
    await expect(page.getByRole("heading", { name: "Checkout" })).toBeVisible();

    await page.goto(`/projects/${slug}/heartbeats/new`);
    await page.getByLabel("Name").fill("internal-etl");
    await page.getByRole("button", { name: "Save heartbeat" }).click();
    await expect(page.getByRole("heading", { name: "internal-etl" })).toBeVisible();

    const visitor = await browser.newContext({ storageState: { cookies: [], origins: [] } });
    const v = await visitor.newPage();
    await v.goto(`/status/${slug}`);
    await expect(v.getByRole("heading", { name: "Storefront" })).toBeVisible();
    // Not checked yet: the page does not claim that everything works.
    await expect(v.getByText("Waiting for the first checks")).toBeVisible();
    await expect(v.getByText("Checkout")).toBeVisible();
    await expect(v.getByText("internal-etl")).toHaveCount(0);
    await expect(v.locator("[data-uptime-bars] > span")).toHaveCount(90);

    const json = await v.request.get(`/api/v1/public/status/${slug}`);
    expect(json.status()).toBe(200);
    expect(json.headers()["access-control-allow-origin"]).toBe("*");
    const body = await json.json();
    expect(body.data.items.map((i: { name: string }) => i.name)).toEqual(["Checkout"]);
    await visitor.close();
  });
});

test.describe("the public pages", () => {
  test.use({ storageState: authFile("tl") });

  test("Uptime reads itself again every 60 s with no Refresh button; an incident row opens to its updates", async ({ page, browser }) => {
    const slug = await apiProject(page, "Live");
    await apiMonitor(page, slug, { name: "Checkout API" });
    const { pingUrl } = await apiHeartbeat(page, slug);
    expect((await page.request.post(`${pingUrl}/fail`)).status()).toBe(200);

    const visitor = await browser.newContext({ storageState: { cookies: [], origins: [] } });
    const v = await visitor.newPage();
    await v.clock.install();
    await v.goto(`/status/${slug}`);
    await expect(v.getByText("Checkout API")).toBeVisible();
    await expect(v.getByRole("button", { name: "Refresh" })).toHaveCount(0);
    const again = v.waitForRequest((r) => r.url().endsWith(`/api/ui/public/${slug}/status`));
    await v.clock.runFor(60_000);
    await again;

    await v.goto(`/status/${slug}/incidents`);
    const row = v.getByRole("listitem").filter({ hasText: "nightly-settlement reported a failure" });
    await expect(row.getByText("Affects nightly-settlement.")).toBeHidden();
    await row.getByText("nightly-settlement reported a failure").click();
    await expect(row.getByText("Affects nightly-settlement.")).toBeVisible();

    await v.goto(`/status/${slug}/uptime`);
    // A new project has no older months, so the window arrows stay off; the picker switches systems.
    await expect(v.getByRole("button", { name: "Earlier months" })).toBeDisabled();
    await v.getByRole("combobox", { name: "System" }).click();
    await v.getByRole("option", { name: "nightly-settlement" }).click();
    await expect(v).toHaveURL(/[?&]kind=heartbeat/);
    await visitor.close();
  });
  test("Past uptime moves three months back for a system that is old enough, and forward again", async ({ browser }) => {
    // seeds/qa.sql: project qa-archive has a public monitor made 200 days ago.
    const visitor = await browser.newContext({ storageState: { cookies: [], origins: [] } });
    const v = await visitor.newPage();
    await v.goto("/status/qa-archive/uptime");
    const earlier = v.getByRole("button", { name: "Earlier months" });
    const later = v.getByRole("button", { name: "Later months" });
    await expect(later).toBeDisabled();
    await earlier.click();
    await expect(v).toHaveURL(/[?&]to=\d+/);
    await expect(later).toBeEnabled();
    await later.click();
    await expect(later).toBeDisabled();
    await visitor.close();
  });

  test("a month with more than three resolved incidents shows three, then all of them", async ({ page, browser }) => {
    const slug = await apiProject(page, "Busy month");
    const monitor = await apiMonitor(page, slug, { name: "Checkout API" });
    for (let n = 1; n <= 4; n++) {
      const { id } = await apiSend<{ id: number }>(page, "POST", `/api/ui/projects/${slug}/incidents`, { title: `Slow checkout ${n}`, items: [`monitor:${monitor}`], severity: "warning", message: "Looking into it." }, 201);
      await apiSend(page, "POST", `/api/ui/projects/${slug}/incidents/${id}/updates`, { label: "resolved", message: "Fixed." }, 204);
    }
    const visitor = await browser.newContext({ storageState: { cookies: [], origins: [] } });
    const v = await visitor.newPage();
    await v.goto(`/status/${slug}/incidents`);
    await expect(v.getByText(/^Slow checkout \d$/)).toHaveCount(3);
    await v.getByRole("button", { name: "Show all 4 incidents" }).click();
    await expect(v.getByText(/^Slow checkout \d$/)).toHaveCount(4);
    await visitor.close();
  });
});
