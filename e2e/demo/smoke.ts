/**
 * Checks the built demo in Chrome, served the way GitHub Pages serves it: target/site under
 * /monita/, a folder's path redirected to its slash, and 404.html for a missing file.
 * Run snapshot.ts first.
 *
 *   bun demo/smoke.ts
 *
 * Env: DEMO_OUT (default ../target/site), DEMO_BASE (default /monita/demo).
 */
import { existsSync, statSync } from "node:fs";
import { join, normalize, resolve } from "node:path";
import { expect } from "@playwright/test";

const ROOT = resolve(import.meta.dir, "../..");
const OUT = process.env["DEMO_OUT"] ?? join(ROOT, "target/site");
const BASE = process.env["DEMO_BASE"] ?? "/monita/demo";
const SITE = BASE.slice(0, BASE.lastIndexOf("/"));

const server = Bun.serve({
  port: 0,
  hostname: "127.0.0.1",
  fetch(req) {
    const path = decodeURIComponent(new URL(req.url).pathname);
    if (!path.startsWith(`${SITE}/`)) return new Response("not found", { status: 404 });
    const file = normalize(join(OUT, path.slice(SITE.length)));
    if (!file.startsWith(OUT)) return new Response("not found", { status: 404 });
    const isFile = (f: string): boolean => existsSync(f) && statSync(f).isFile();
    if (isFile(file)) return new Response(Bun.file(file));
    if (!path.endsWith("/") && isFile(join(file, "index.html"))) return Response.redirect(`${path}/${new URL(req.url).search}`, 301);
    if (isFile(join(file, "index.html"))) return new Response(Bun.file(join(file, "index.html")));
    return new Response(Bun.file(join(OUT, "404.html")), { status: 404, headers: { "content-type": "text/html" } });
  },
});
const ORIGIN = `http://127.0.0.1:${server.port}`;

const { chromium } = await import("@playwright/test");
const browser = await chromium.launch({ channel: "chrome" });
const problems: string[] = [];
try {
  const context = await browser.newContext({ viewport: { width: 1440, height: 900 } });
  const page = await context.newPage();
  page.on("console", (m) => {
    if (m.type() === "error") problems.push(`console: ${m.text()}`);
  });
  page.on("pageerror", (e) => problems.push(`page error: ${e.message}`));
  page.on("response", (r) => {
    const url = new URL(r.url());
    if (url.pathname.startsWith("/api/ui/") && r.status() === 404) problems.push(`no recording: ${url.pathname}${url.search}`);
  });

  // A first visit: the worker starts, then the app draws the projects.
  await page.goto(`${ORIGIN}${BASE}/`);
  await expect(page.getByRole("note")).toContainText("read-only");
  await page.getByRole("radio", { name: "Card view" }).click();
  await expect(page.locator('a[href^="/monita/demo/projects/"]').first()).toBeVisible();
  await page.getByRole("radio", { name: "List view" }).click();
  const row = page.locator("tbody tr").first();
  await expect(row).toBeVisible();
  await row.locator("td").nth(1).click();
  await expect(page).toHaveURL(new RegExp(`${BASE}/projects/[a-z-]+$`));
  await page.getByRole("radio", { name: "Chart view" }).click().catch(() => {});

  // A link straight to a monitor: Pages adds a slash, the app reads it without.
  await page.goto(`${ORIGIN}${BASE}/projects/portal/monitors/2`);
  await expect(page).toHaveURL(`${ORIGIN}${BASE}/projects/portal/monitors/2`);
  await expect(page.getByRole("heading").first()).toBeVisible();
  for (const range of await page.locator('[data-slot="toggle-group-item"]').all()) await range.click();

  // A write gets the read-only answer, and the banner says so.
  const status = await page.evaluate(() => fetch("/api/ui/projects/portal/monitors/2/pause", { method: "POST" }).then((r) => r.status));
  if (status !== 403) problems.push(`a write answered ${status}, not 403`);
  await expect(page.getByRole("note")).toContainText("cannot save changes");

  // A status page, and a demo page with no file of its own.
  await page.goto(`${ORIGIN}${BASE}/status/payments`);
  await expect(page.getByRole("heading").first()).toBeVisible();
  await page.goto(`${ORIGIN}${BASE}/projects/portal/nothing-here`);
  await expect(page).toHaveURL(`${ORIGIN}${BASE}/projects/portal/nothing-here`);
  await expect(page.getByRole("heading", { name: "Not found" })).toBeVisible();
} catch (e) {
  problems.push(String(e));
} finally {
  await browser.close();
  server.stop(true);
}
// Chrome logs the write's 403, and the not-found page's own 404, as failed loads.
const real = problems.filter((p) => !/^console: Failed to load resource: the server responded with a status of (403|404)/.test(p));
if (real.length > 0) {
  console.error(`demo smoke: ${real.length} problems\n${real.join("\n")}`);
  process.exit(1);
}
console.log("demo smoke: passed");
