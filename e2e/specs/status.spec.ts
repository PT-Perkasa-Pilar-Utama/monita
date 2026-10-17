import { expect, test } from "@playwright/test";
import { createProject } from "../lib/project";
import { authFile } from "../lib/roles";

test.describe("as the TL", () => {
  test.use({ storageState: authFile("tl") });

  test("@US-21 @US-22 a visitor sees public items only, with 90 day bars, and the same data as JSON", async ({ page, browser }) => {
    const slug = `s-${Date.now()}`;
    await createProject(page, "Storefront", slug);

    await page.goto(`/projects/${slug}/monitors/new`);
    await page.getByLabel("Name").fill("Checkout");
    await page.getByLabel("URL").fill("http://127.0.0.1:18080/healthz");
    await page.getByLabel("Show on the public status page").check();
    await page.getByRole("button", { name: "Save monitor" }).click();

    await page.goto(`/projects/${slug}/heartbeats/new`);
    await page.getByLabel("Name").fill("internal-etl");
    await page.getByRole("button", { name: "Save heartbeat" }).click();

    const visitor = await browser.newContext({ storageState: { cookies: [], origins: [] } });
    const v = await visitor.newPage();
    await v.goto(`/status/${slug}`);
    await expect(v.getByRole("heading", { name: "Storefront" })).toBeVisible();
    await expect(v.getByText("All systems operational")).toBeVisible();
    await expect(v.getByText("Checkout")).toBeVisible();
    await expect(v.getByText("internal-etl")).toHaveCount(0);
    await expect(v.locator(".uptime__bars > span")).toHaveCount(90);

    const json = await v.request.get(`/api/v1/public/status/${slug}`);
    expect(json.status()).toBe(200);
    expect(json.headers()["access-control-allow-origin"]).toBe("*");
    const body = await json.json();
    expect(body.data.items.map((i: { name: string }) => i.name)).toEqual(["Checkout"]);
    await visitor.close();
  });
});
