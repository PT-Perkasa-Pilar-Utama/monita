/**
 * Builds the static demo for GitHub Pages: boots the real binary on seeds/demo.sql, crawls every
 * page a super admin and the public can GET, and saves them with relative links under
 * target/site/demo. Forms and filtered lists point at "#demo-off", which demo.js answers.
 * Run site/build.ts first: it writes the rest of target/site, and this checks every link in it.
 *
 *   bun demo/snapshot.ts                  # build target/site/demo, then check every link
 *   bun demo/snapshot.ts --screens <dir>  # also save the README screenshots there
 *
 * Env: DEMO_BIN (default ../target/debug/monita), DEMO_OUT (default ../target/site).
 */
import { copyFileSync, mkdirSync, mkdtempSync, readdirSync, readFileSync, rmSync, statSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { DEMO_ORIGIN, OFF, assetLinks, fileFor, pageLinks, relative, rewrite } from "../lib/demo-site";

const ROOT = resolve(import.meta.dir, "../..");
const BIN = process.env["DEMO_BIN"] ?? join(ROOT, "target/debug/monita");
const OUT = process.env["DEMO_OUT"] ?? join(ROOT, "target/site");
const SITE = join(OUT, "demo");
const PORT = 18190;
const LOCAL = `http://127.0.0.1:${PORT}`;
const TOKEN = "demo-snapshot";
const START = ["/", "/status/portal", "/status/payments"];
const MAX_PAGES = 400;

async function main(): Promise<void> {
  const screens = process.argv.indexOf("--screens");
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
    const pages = await crawl(cookie);
    await write(pages);
    if (screens !== -1) await screenshots(cookie, process.argv[screens + 1] ?? "screens");
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
  const res = await fetch(`${LOCAL}/login`, {
    method: "POST",
    redirect: "manual",
    headers: { "content-type": "application/x-www-form-urlencoded", origin: DEMO_ORIGIN },
    body: new URLSearchParams({ email: "admin@monita.test", password: "monita-dev-password" }),
  });
  const set = res.headers.getSetCookie().find((c) => c.startsWith("monita_session="));
  if (res.status !== 303 || set === undefined) throw new Error(`sign-in returned ${res.status}`);
  return set.split(";")[0]!;
}

/** Every page reachable by GET from START, as path -> HTML. */
async function crawl(cookie: string): Promise<Map<string, string>> {
  const pages = new Map<string, string>();
  const queue = [...START];
  const seen = new Set(queue);
  while (queue.length > 0) {
    const path = queue.shift()!;
    const res = await fetch(LOCAL + path, { redirect: "manual", headers: { cookie } });
    if (res.status !== 200 || !(res.headers.get("content-type") ?? "").startsWith("text/html")) continue;
    const html = await res.text();
    pages.set(path, html);
    if (pages.size > MAX_PAGES) throw new Error(`more than ${MAX_PAGES} pages; is a link looping?`);
    for (const link of pageLinks(html)) {
      if (!seen.has(link)) {
        seen.add(link);
        queue.push(link);
      }
    }
  }
  return pages;
}

const BANNER = `<aside class="demo-banner" role="note"><strong>Demo</strong> A read-only snapshot of Monita on sample data.
<a href="https://github.com/PT-Perkasa-Pilar-Utama/monita#install">Install it</a> to add monitors and post updates.</aside>`;

async function write(pages: Map<string, string>): Promise<void> {
  // site/build.ts owns the rest of OUT; the demo replaces only its own folder.
  rmSync(SITE, { recursive: true, force: true });
  const paths = new Set(pages.keys());
  const assets = new Set<string>();
  for (const [path, html] of pages) {
    for (const a of assetLinks(html)) assets.add(a);
    const up = relative(path, "/", true);
    const page = rewrite(html, path, paths)
      .replace("</head>", `<link rel="stylesheet" href="${up}_demo/demo.css">\n</head>`)
      .replace(/<body([^>]*)>/, `<body$1>\n${BANNER}`)
      .replace("</body>", `<script src="${up}_demo/demo.js" defer></script>\n</body>`);
    save(join(SITE, fileFor(path)), page);
  }
  for (const a of assets) await saveAsset(a);
  for (const f of ["demo.css", "demo.js"]) {
    mkdirSync(join(SITE, "_demo"), { recursive: true });
    copyFileSync(join(import.meta.dir, f), join(SITE, "_demo", f));
  }
}

function save(file: string, body: string | Uint8Array): void {
  mkdirSync(dirname(file), { recursive: true });
  writeFileSync(file, body);
}

async function saveAsset(path: string): Promise<void> {
  const res = await fetch(LOCAL + path);
  if (!res.ok) throw new Error(`${path} returned ${res.status}`);
  if (path.endsWith(".css")) {
    const css = await res.text();
    for (const [, font] of css.matchAll(/url\("(\/static\/[^"]+)"\)/g)) await saveAsset(font!);
    save(join(SITE, path), css.replaceAll('url("/static/', 'url("./'));
  } else {
    save(join(SITE, path), new Uint8Array(await res.arrayBuffer()));
  }
}

/** Relative links in the built site that point at no file, and any trace of the local server. */
function check(dir: string): string[] {
  const broken: string[] = [];
  const walk = (d: string): string[] =>
    readdirSync(d).flatMap((n) => (statSync(join(d, n)).isDirectory() ? walk(join(d, n)) : [join(d, n)]));
  for (const file of walk(dir).filter((f) => f.endsWith(".html"))) {
    const html = readFileSync(file, "utf8");
    // The docs explain local runs; only a crawled page must not leak the snapshot server.
    if (file.startsWith(SITE) && html.includes(LOCAL)) broken.push(`${file}: names the local server`);
    for (const [, url] of html.matchAll(/\b(?:href|src|action)="([^"]*)"/g)) {
      if (url === OFF || url!.startsWith("#") || /^[a-z]+:/i.test(url!)) continue;
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

/** The README screenshots, from the live server so they match the demo. */
async function screenshots(cookie: string, dir: string): Promise<void> {
  const { chromium } = await import("@playwright/test");
  const browser = await chromium.launch({ channel: "chrome" });
  try {
    const context = await browser.newContext({ viewport: { width: 1440, height: 900 }, colorScheme: "light", deviceScaleFactor: 2 });
    const [name, value] = cookie.split("=") as [string, string];
    await context.addCookies([{ name, value, domain: "127.0.0.1", path: "/" }]);
    const page = await context.newPage();
    mkdirSync(dir, { recursive: true });
    for (const [file, path] of [
      ["monitor.png", "/projects/portal/monitors/2"],
      ["status.png", "/status/payments"],
      ["monitors.png", "/projects/portal/monitors"],
      ["tickets.png", "/projects/portal/tickets"],
    ] as const) {
      await page.goto(LOCAL + path);
      await page.screenshot({ path: join(dir, file) });
    }
  } finally {
    await browser.close();
  }
}

await main();
