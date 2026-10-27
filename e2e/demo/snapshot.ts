/**
 * Builds the online demo for GitHub Pages under target/site/demo: the real web app, built for the
 * demo's path, with a service worker that answers its API from a recording. To record, this boots
 * the debug binary on seeds/demo.sql, opens every page a super admin and the public can reach in
 * headless Chrome, works each page's views, tabs, filters and pages, and keeps every API answer.
 * Run site/build.ts first: it writes the rest of target/site, and this checks every link in it.
 * Build the app for the demo first too: `MONITA_BASE=/monita/demo bun run build.ts` in web/.
 *
 *   bun demo/snapshot.ts                  # build target/site/demo, then check every link
 *   bun demo/snapshot.ts --screens <dir>  # also save the README screenshots there
 *
 * Env: DEMO_BIN (default ../target/debug/monita), DEMO_OUT (default ../target/site),
 * DEMO_BASE (default /monita/demo, the path web/dist-demo was built for).
 */
import { cpSync, existsSync, mkdirSync, mkdtempSync, readdirSync, readFileSync, rmSync, statSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import type { Browser, BrowserContext, Locator, Page } from "@playwright/test";
import { DEMO_ORIGIN, type Entry, type Recording, fileFor, pageLinks, recordable } from "../lib/demo-site";

const ROOT = resolve(import.meta.dir, "../..");
const BIN = process.env["DEMO_BIN"] ?? join(ROOT, "target/debug/monita");
const OUT = process.env["DEMO_OUT"] ?? join(ROOT, "target/site");
const BASE = process.env["DEMO_BASE"] ?? "/monita/demo";
const WEB = join(ROOT, "web/dist-demo");
const SITE = join(OUT, "demo");
const PORT = 18190;
const LOCAL = `http://127.0.0.1:${PORT}`;
const TOKEN = "demo-snapshot";
const START = ["/", "/status/portal", "/status/payments"];
const MAX_PAGES = 400;
// The recording's clock: chart labels and day bars follow the viewer's offset, sent with each request.
const ZONE = "Asia/Jakarta";

async function main(): Promise<void> {
  const screens = process.argv.indexOf("--screens");
  if (!existsSync(join(WEB, "index.html"))) throw new Error(`${WEB} is missing: run MONITA_BASE=${BASE} bun run build.ts in web/`);
  const dbDir = mkdtempSync(join(tmpdir(), "monita-demo-"));
  const server = Bun.spawn([BIN, "serve"], {
    env: {
      PATH: process.env["PATH"] ?? "",
      MONITA_ENV: "test",
      MONITA_BIND: `127.0.0.1:${PORT}`,
      MONITA_BASE_URL: DEMO_ORIGIN,
      MONITA_DB_PATH: join(dbDir, "monita.db"),
      MONITA_RESET_TOKEN: TOKEN,
      MONITA_LOG: "warn",
    },
    stdout: "inherit",
    stderr: "inherit",
  });
  try {
    await waitHealthy();
    const reset = await fetch(`${LOCAL}/admin/reset-state`, {
      method: "POST",
      headers: { "content-type": "application/json", "x-reset-token": TOKEN },
      body: JSON.stringify({ seed: "demo" }),
    });
    if (reset.status !== 204) throw new Error(`reset-state returned ${reset.status}`);
    const cookie = await signIn();
    const { chromium } = await import("@playwright/test");
    const browser = await chromium.launch({ channel: "chrome" });
    try {
      const capturedAt = Math.floor(Date.now() / 1000);
      const context = await signedIn(browser, cookie, 1);
      const entries = record(context);
      const pages = await crawl(context);
      await context.close();
      await write(pages, { capturedAt, entries: Object.fromEntries([...(await entries())].sort(([a], [b]) => a.localeCompare(b))) });
      if (screens !== -1) await screenshots(browser, cookie, process.argv[screens + 1] ?? "screens");
    } finally {
      await browser.close();
    }
  } finally {
    server.kill();
    await server.exited;
    rmSync(dbDir, { recursive: true, force: true });
  }
  const broken = check(OUT);
  if (broken.length > 0) {
    console.error(`demo: ${broken.length} broken links\n${broken.slice(0, 20).join("\n")}`);
    process.exit(1);
  }
}

async function waitHealthy(): Promise<void> {
  for (let i = 0; i < 50; i++) {
    const ok = await fetch(`${LOCAL}/healthz`).then((r) => r.ok, () => false);
    if (ok) return;
    await Bun.sleep(200);
  }
  throw new Error(`${BIN} did not answer ${LOCAL}/healthz`);
}

async function signIn(): Promise<string> {
  const res = await fetch(`${LOCAL}/api/ui/login`, {
    method: "POST",
    headers: { "content-type": "application/json", origin: DEMO_ORIGIN },
    body: JSON.stringify({ email: "admin@monita.test", password: "monita-dev-password" }),
  });
  const set = res.headers.getSetCookie().find((c) => c.startsWith("monita_session="));
  if (res.status !== 204 || set === undefined) throw new Error(`sign-in returned ${res.status}`);
  return set.split(";")[0]!;
}

/** A light-scheme browser context with the super admin's session. */
async function signedIn(browser: Browser, cookie: string, scale: number): Promise<BrowserContext> {
  const context = await browser.newContext({ viewport: { width: 1440, height: 900 }, colorScheme: "light", deviceScaleFactor: scale, timezoneId: ZONE });
  const [name, value] = cookie.split("=") as [string, string];
  await context.addCookies([{ name, value, domain: "127.0.0.1", path: "/" }]);
  // The demo and the screenshots show the pages, not the notes a first visit gets. boot.js does the same.
  await context.addInitScript(() => {
    localStorage.setItem("monita.2fa-note-closed:admin@monita.test", "1");
    localStorage.setItem("monita.whats-new-seen:admin@monita.test", "999.0.0");
  });
  return context;
}

/** Keeps every answer the demo serves (recordable paths, GET) from `context`; call the result when done. */
function record(context: BrowserContext): () => Promise<Map<string, Entry>> {
  const entries = new Map<string, Entry>();
  const pending: Promise<void>[] = [];
  context.on("response", (res) => {
    const url = new URL(res.url());
    // A 304 has no body: the 200 before it is kept.
    if (url.origin !== LOCAL || res.request().method() !== "GET" || !recordable(url.pathname) || res.status() === 304) return;
    pending.push(
      res.body().then(
        (body) => {
          const type = res.headers()["content-type"] ?? "application/octet-stream";
          const text = /json|text|svg/.test(type);
          entries.set(url.pathname + url.search, { status: res.status(), type, body: text ? body.toString("utf8") : body.toString("base64"), ...(text ? {} : { base64: true as const }) });
        },
        () => {},
      ),
    );
  });
  return async () => {
    await Promise.all(pending);
    return entries;
  };
}

/** Waits until the page's data has loaded. */
async function settle(page: Page): Promise<void> {
  await page.waitForLoadState("networkidle");
  await page.waitForFunction(() => !document.querySelector('[data-slot="skeleton"]'), null, { timeout: 10_000 }).catch(() => {});
}

const pathOf = (page: Page): string => new URL(page.url()).pathname.replace(/\/+$/, "") || "/";

/** Opens `path`; false when the app answers Not found. */
async function visit(page: Page, path: string): Promise<boolean> {
  await page.goto(LOCAL + path);
  await settle(page);
  return (await page.getByRole("heading", { name: "Not found", exact: true }).count()) === 0;
}

/**
 * Works the controls on `path` that only read: views, tabs, filter menus and the pager, so their
 * answers are recorded. A click that leads to another page adds it to `found` and comes back.
 */
async function explore(page: Page, path: string, found: Set<string>): Promise<void> {
  const back = async (): Promise<void> => {
    if (pathOf(page) === path) return;
    found.add(pathOf(page));
    await visit(page, path);
  };
  // A click the page does not take in time (a poll redrew the control) is skipped.
  const tap = (target: Locator): Promise<boolean> => target.click({ timeout: 2_000 }).then(() => true, () => false);
  // A route that opens a panel works the panel's controls; the page under it takes no clicks.
  const dialog = page.locator('[role="dialog"][data-state="open"]');
  const scope = (await dialog.count()) > 0 ? dialog.first() : page.locator("body");
  const toggles = scope.locator('[data-slot="toggle-group-item"]:not([disabled]), [role="tab"]:not([disabled])');
  for (let i = 0; i < Math.min(await toggles.count(), 20); i++) {
    if (!(await toggles.nth(i).isVisible()) || !(await tap(toggles.nth(i)))) continue;
    await settle(page);
    await back();
  }
  const menus = scope.locator('[aria-haspopup="menu"]');
  const options = page.locator('[role="menu"][data-state="open"] [role="menuitemradio"]');
  for (let i = 0; i < Math.min(await menus.count(), 8); i++) {
    const open = async (): Promise<boolean> => {
      // A menu still closing ignores the click that would open it again.
      await page.locator('[role="menu"]').waitFor({ state: "detached", timeout: 2_000 }).catch(() => {});
      if (!(await menus.nth(i).isVisible()) || !(await tap(menus.nth(i)))) return false;
      return page.locator('[role="menu"][data-state="open"]').waitFor({ timeout: 2_000 }).then(() => true, () => false);
    };
    if (!(await open())) continue;
    const n = Math.min(await options.count(), 12);
    await page.keyboard.press("Escape");
    // Each option, then the first again, which is "All" on a filter.
    for (const j of [...Array(n).keys()].slice(1).concat(n > 0 ? [0] : [])) {
      if (!(await open())) break;
      if (!(await tap(options.nth(j)))) await page.keyboard.press("Escape");
      await settle(page);
      await back();
    }
  }
  for (let turns = 0; turns < 10; turns++) {
    const next = scope.getByRole("button", { name: /^(Next|Older)\b/ }).first();
    if (!(await next.count()) || !(await next.isVisible()) || (await next.isDisabled()) || !(await tap(next))) break;
    await settle(page);
    await back();
  }
}

/** Every page reachable from START by a link or a control, each one worked by explore(). */
async function crawl(context: BrowserContext): Promise<string[]> {
  const page = await context.newPage();
  const pages: string[] = [];
  const queue = [...START];
  const seen = new Set(queue);
  const found = new Set<string>();
  while (queue.length > 0) {
    const path = queue.shift()!;
    if (!(await visit(page, path))) continue;
    pages.push(path);
    if (pages.length > MAX_PAGES) throw new Error(`more than ${MAX_PAGES} pages; is a link looping?`);
    found.clear();
    for (const link of pageLinks(await page.content())) found.add(link);
    await explore(page, path, found);
    for (const link of found) {
      if (!seen.has(link)) {
        seen.add(link);
        queue.push(link);
      }
    }
  }
  return pages;
}

/** The app's page with boot.js in place of its module script, so the worker starts first. */
function shell(html: string): string {
  const out = html.replace(/<script type="module" src="([^"]+)"><\/script>/, (_, entry: string) => `<script src="${BASE}/_demo/boot.js" data-base="${BASE}" data-entry="${entry}"></script>`);
  if (out === html) throw new Error("the built page has no module script to replace");
  return out;
}

/** Writes the demo: the app, its static files, a page file per crawled path, the worker and the recording. */
async function write(pages: string[], recording: Recording): Promise<void> {
  // site/build.ts owns the rest of OUT; the demo replaces only its own folder.
  rmSync(SITE, { recursive: true, force: true });
  mkdirSync(join(SITE, "_demo"), { recursive: true });
  cpSync(join(WEB, "assets"), join(SITE, "assets"), { recursive: true });
  cpSync(join(ROOT, "static"), join(SITE, "static"), { recursive: true });
  const team = shell(readFileSync(join(WEB, "index.html"), "utf8"));
  const status = shell(readFileSync(join(WEB, "status.html"), "utf8"));
  writeFileSync(join(SITE, "status.html"), status);
  for (const path of pages) save(join(SITE, fileFor(path)), path.startsWith("/status/") ? status : team);
  const json = JSON.stringify(recording);
  const api = `api-${new Bun.CryptoHasher("sha256").update(json).digest("hex").slice(0, 12)}.json`;
  writeFileSync(join(SITE, "_demo", api), json);
  writeFileSync(join(SITE, "_demo", "boot.js"), readFileSync(join(import.meta.dir, "boot.js")));
  // Pages answers a path it has no file for with the site's 404.html: a demo path goes to the app.
  if (!existsSync(join(OUT, "404.html"))) writeFileSync(join(OUT, "404.html"), notFound());
  await bundleWorker(api);
  console.log(`demo: ${pages.length} pages, ${Object.keys(recording.entries).length} recorded answers (${(json.length / 1e6).toFixed(1)} MB)`);
}

async function bundleWorker(api: string): Promise<void> {
  const out = await Bun.build({
    entrypoints: [join(import.meta.dir, "sw.ts")],
    target: "browser",
    minify: true,
    define: { DEMO_BASE: JSON.stringify(BASE), DEMO_API: JSON.stringify(api) },
  });
  if (!out.success) throw new Error(out.logs.join("\n"));
  // A worker controls only the pages under its own folder, so it sits at the demo's root.
  writeFileSync(join(SITE, "sw.js"), await out.outputs[0]!.text());
}

function notFound(): string {
  return `<!doctype html>
<html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1"><title>Not found</title>
<script>
  // A demo page with no file of its own, such as a filtered list: the app shows it.
  const base = ${JSON.stringify(BASE)};
  if (location.pathname.startsWith(base + "/")) {
    const shell = location.pathname.startsWith(base + "/status/") ? "/status.html" : "/";
    location.replace(base + shell + "?demo-to=" + encodeURIComponent(location.pathname + location.search + location.hash));
  }
</script></head>
<body style="font-family:system-ui,sans-serif;padding:16px"><h1>Not found</h1><p>This page does not exist. <a href="${dirname(BASE)}/">Go to the Monita site</a>.</p></body></html>
`;
}

function save(file: string, body: string): void {
  mkdirSync(dirname(file), { recursive: true });
  writeFileSync(file, body);
}

/** Relative links in the built site outside the demo that point at no file. The demo is an app: its smoke test checks it. */
function check(dir: string): string[] {
  const broken: string[] = [];
  const walk = (d: string): string[] =>
    readdirSync(d).flatMap((n) => (statSync(join(d, n)).isDirectory() ? walk(join(d, n)) : [join(d, n)]));
  for (const file of walk(dir).filter((f) => f.endsWith(".html") && !f.startsWith(SITE + "/") && f !== join(dir, "404.html"))) {
    const html = readFileSync(file, "utf8");
    for (const [, url] of html.matchAll(/\b(?:href|src|action)="([^"]*)"/g)) {
      if (url!.startsWith("#") || /^[a-z]+:/i.test(url!)) continue;
      if (url!.startsWith("/")) {
        broken.push(`${file}: absolute ${url}`);
        continue;
      }
      const path = url!.replace(/[?#].*$/, "");
      const target = resolve(dirname(file), path);
      const exists = (f: string) => {
        try {
          return statSync(f).isFile();
        } catch {
          return false;
        }
      };
      if (!exists(target) && !exists(join(target, "index.html"))) broken.push(`${file}: ${url}`);
    }
  }
  return broken;
}

/** The README and landing-page screenshots, from the live server so they match the demo, and the share image. */
async function screenshots(browser: Browser, cookie: string, dir: string): Promise<void> {
  const context = await signedIn(browser, cookie, 2);
  const page = await context.newPage();
  mkdirSync(dir, { recursive: true });
  for (const [file, path] of [
    ["monitor.png", "/projects/portal/monitors/2"],
    ["status.png", "/status/payments"],
    ["monitors.png", "/projects/portal/monitors"],
    ["tickets.png", "/projects/portal/tickets"],
    ["incident.png", "/projects/payments/incidents/6"],
    ["heartbeat.png", "/projects/portal/heartbeats/1"],
    ["history.png", "/status/payments/history"],
    ["changelog.png", "/status/portal/changelog"],
  ] as const) {
    await visit(page, path);
    await page.screenshot({ path: join(dir, file) });
  }
  // The status page as its users mostly read it: on a phone. The landing page's hero shows it.
  const phone = await browser.newPage({ viewport: { width: 390, height: 760 }, deviceScaleFactor: 3, colorScheme: "light" });
  await visit(phone, "/status/payments");
  await phone.screenshot({ path: join(dir, "status-phone.png") });
  await phone.close();
  // The 1200x630 link-preview image, drawn from site/og.html and the monitors screenshot above.
  const og = await browser.newPage({ viewport: { width: 1200, height: 630 }, deviceScaleFactor: 1, colorScheme: "light" });
  await og.goto(`file://${join(ROOT, "site/og.html")}`);
  await og.waitForLoadState("networkidle");
  await og.screenshot({ path: join(dir, "og.png") });
}

await main();
