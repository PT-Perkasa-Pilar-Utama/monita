/**
 * Builds the project site into target/site: the landing page, the docs from docs/guide/*.md, and
 * the shared assets. e2e/demo/snapshot.ts then adds target/site/demo and checks every link.
 *
 *   bun site/build.ts
 *
 * Markdown goes through Bun.markdown; nothing is installed.
 */
import { createHash } from "node:crypto";
import { copyFileSync, mkdirSync, readdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { highlight } from "./highlight";

const ROOT = resolve(import.meta.dir, "..");
const OUT = process.env["DEMO_OUT"] ?? join(ROOT, "target/site");
const GUIDE = join(ROOT, "docs/guide");
const REPO = "https://github.com/PT-Perkasa-Pilar-Utama/monita";
const SITE_URL = "https://pt-perkasa-pilar-utama.github.io/monita/";

/** Link-preview tags. Chat apps and social sites fetch the image by its absolute URL. */
export function ogTags(title: string, description: string, path: string): string {
  return `<meta property="og:type" content="website">
<meta property="og:site_name" content="Monita">
<meta property="og:title" content="${esc(title)}">
<meta property="og:description" content="${esc(description)}">
<meta property="og:url" content="${SITE_URL}${path}">
<meta property="og:image" content="${SITE_URL}assets/og.png">
<meta property="og:image:width" content="1200">
<meta property="og:image:height" content="630">
<meta property="og:image:alt" content="Monita: find out a service is down before your users write to you.">
<meta name="twitter:card" content="summary_large_image">`;
}

type Page = { slug: string; title: string; lead: string; body: string; headings: [string, string][] };
type Group = { title: string; icon: string; blurb: string; pages: string[] };

/** The sidebar, in reading order. Every guide file belongs to one group. */
const GROUPS: Group[] = [
  { title: "Start", icon: "play", blurb: "What Monita does, a first monitor on your laptop, and a production install.", pages: ["getting-started", "install"] },
  { title: "Watch", icon: "pulse", blurb: "HTTP monitors for URLs and heartbeats for cron jobs.", pages: ["monitors", "heartbeats"] },
  { title: "Respond", icon: "bell", blurb: "Incidents, alerts, the public status page and bug tickets.", pages: ["incidents", "status-page", "tickets"] },
  { title: "Operate", icon: "wrench", blurb: "Configuration, the command line, backups, upgrades and limits.", pages: ["configuration", "cli", "operations", "limits"] },
];

const ICONS: Record<string, string> = {
  play: '<path d="M5 3.5v9l7-4.5z"/>',
  pulse: '<path d="M1.5 8.5h3l1.5-4 3 7 1.5-3h4"/>',
  bell: '<path d="M4 11V7a4 4 0 0 1 8 0v4l1 1.5H3zM6.5 14h3"/>',
  wrench: '<path d="M10 2.5a3.5 3.5 0 0 0-3.2 4.9L2.5 11.7 4.3 13.5l4.3-4.3A3.5 3.5 0 0 0 13.5 6l-2 2-2-.5-.5-2z"/>',
  book: '<path d="M2 3h4.5A1.5 1.5 0 0 1 8 4.5V13a1.5 1.5 0 0 0-1.5-1.5H2zM14 3H9.5A1.5 1.5 0 0 0 8 4.5V13a1.5 1.5 0 0 1 1.5-1.5H14z"/>',
  arrow: '<path d="M3 8h10M9 4l4 4-4 4"/>',
  search: '<circle cx="7" cy="7" r="4.5"/><path d="M10.5 10.5 14 14"/>',
  github: '<path d="M8 1.5a6.5 6.5 0 0 0-2 12.7c.3 0 .5-.2.5-.4v-1.3c-1.8.4-2.2-.8-2.2-.8-.3-.8-.7-1-.7-1-.6-.4 0-.4 0-.4.7 0 1 .7 1 .7.6 1 1.6.7 2 .5 0-.4.2-.7.4-.9-1.4-.2-3-.7-3-3.2 0-.7.3-1.3.7-1.7-.1-.2-.3-.8 0-1.7 0 0 .6-.2 1.8.7a6 6 0 0 1 3.2 0c1.2-.9 1.8-.7 1.8-.7.3.9.1 1.5 0 1.7.4.4.7 1 .7 1.7 0 2.5-1.6 3-3 3.2.2.2.4.6.4 1.2v1.8c0 .2.2.4.5.4A6.5 6.5 0 0 0 8 1.5z"/>',
  theme: '<circle cx="8" cy="8" r="6"/><path d="M8 2a6 6 0 0 0 0 12z" fill="currentColor"/>',
  mark: '<circle cx="10" cy="10" r="8.25"/><path d="M4.5 10.5h2.5l1.5-3.5 3 6 1.5-2.5h2.5"/>',
};

function icon(name: string, cls = "icon"): string {
  const box = name === "mark" ? "0 0 20 20" : "0 0 16 16";
  return `<svg class="${cls}" viewBox="${box}" aria-hidden="true">${ICONS[name]}</svg>`;
}

function esc(s: string): string {
  return s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;").replace(/"/g, "&quot;");
}

function slugify(text: string): string {
  return text.toLowerCase().replace(/<[^>]+>/g, "").replace(/&[a-z]+;/g, "").replace(/[^a-z0-9]+/g, "-").replace(/^-|-$/g, "");
}

/** A guide file as a page: links between guides become site links, others point at GitHub. */
function load(slug: string): Page {
  const md = readFileSync(join(GUIDE, `${slug}.md`), "utf8");
  const title = /^# (.+)$/m.exec(md)?.[1] ?? slug;
  const rest = md.replace(/^# .+\n+/, "");
  const lead = rest.split(/\n\n/)[0]!.trim();
  let body = Bun.markdown.html(rest.slice(lead.length).trim());
  const headings: [string, string][] = [];
  body = body.replace(/<h2>(.*?)<\/h2>/g, (_, inner: string) => {
    const id = slugify(inner);
    headings.push([id, inner]);
    return `<h2 id="${id}"><a class="anchor" href="#${id}">${inner}</a></h2>`;
  });
  body = links(body);
  body = body.replace(/<table>/g, '<div class="table-scroll"><table>').replace(/<\/table>/g, "</table></div>");
  const leadHtml = links(Bun.markdown.html(lead)).replace(/^<p>|<\/p>\s*$/g, "");
  return { slug, title, lead: leadHtml, body, headings };
}

/** A link to another guide becomes a site link; a link to any other repo file points at GitHub. */
function links(html: string): string {
  return html
    .replace(/href="([^"#:]+)\.md(#[^"]*)?"/g, (_, target: string, hash = "") => {
      if (/^[a-z0-9-]+$/.test(target)) return `href="../${target}/${hash}"`;
      const path = resolve(GUIDE, `${target}.md`).slice(ROOT.length + 1);
      return `href="${REPO}/blob/main/${path}${hash}"`;
    })
    .replace(/href="\.\.\/\.\.\/([^"]+)"/g, (_, path: string) => `href="${REPO}/blob/main/${path}"`);
}

function shell(opts: { title: string; up: string; active: string; side: string; main: string; toc: string; description: string; path: string }): string {
  const { up } = opts;
  return `<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>${esc(opts.title)}</title>
<meta name="description" content="${esc(opts.description)}">
${ogTags(opts.title, opts.description, opts.path)}
<link rel="icon" href="${up}assets/favicon.svg" type="image/svg+xml">
<script src="${up}assets/theme.js"></script>
<link rel="stylesheet" href="${up}assets/app.css">
<link rel="stylesheet" href="${up}assets/site.css">
<script src="${up}assets/app.js" defer></script>
<script src="${up}assets/docs.js" defer></script>
</head>
<body class="site">
${topbar(up, opts.active)}
<div class="docs">
  <aside class="docs__side" id="docs-nav">${opts.side}</aside>
  <main class="docs__main">${opts.main}</main>
  <nav class="docs__toc" aria-label="On this page">${opts.toc}</nav>
</div>
</body>
</html>
`;
}

export function topbar(up: string, active: string): string {
  const link = (href: string, label: string, key: string, iconName: string) =>
    `<a class="topbar__link" href="${href}"${active === key ? ' aria-current="page"' : ""}>${icon(iconName)}${label}</a>`;
  return `<header class="topbar">
  <a class="topbar__brand" href="${up}" aria-label="Monita">${icon("mark", "topbar__mark")}<span>Monita</span></a>
  <nav class="topbar__nav" aria-label="Site">
    ${link(`${up}demo/`, "Demo", "demo", "pulse")}
    ${link(`${up}docs/`, "Documentation", "docs", "book")}
    <a class="topbar__link" href="${REPO}">${icon("github", "icon icon--fill")}GitHub</a>
    <button class="topbar__icon" type="button" data-theme-toggle aria-label="Switch light or dark theme" title="Switch theme">${icon("theme")}</button>
  </nav>
</header>`;
}

function sidebar(pages: Map<string, Page>, up: string, current: string): string {
  const groups = GROUPS.map((g) => {
    const items = g.pages
      .map((slug) => {
        const p = pages.get(slug)!;
        const terms = esc([p.title, ...p.headings.map(([, h]) => h.replace(/<[^>]+>/g, ""))].join(" ").toLowerCase());
        return `<li><a href="${up}docs/${slug}/" data-terms="${terms}"${slug === current ? ' aria-current="page"' : ""}>${esc(p.title)}</a></li>`;
      })
      .join("");
    return `<section class="side__group"><h2 class="side__title">${icon(g.icon)}${g.title}</h2><ul>${items}</ul></section>`;
  }).join("");
  return `<label class="side__search">${icon("search")}<input type="search" placeholder="Search" aria-label="Search the docs" data-docs-search><kbd>/</kbd></label>
<a class="side__overview" href="${up}docs/"${current === "" ? ' aria-current="page"' : ""}>${icon("book")}Overview</a>
${groups}
<p class="side__empty" hidden>No page matches.</p>`;
}

function toc(headings: [string, string][]): string {
  if (headings.length === 0) return "";
  return `<h2 class="toc__title">On this page</h2><ul>${headings.map(([id, h]) => `<li><a href="#${id}">${h}</a></li>`).join("")}</ul>`;
}

function pageHtml(pages: Map<string, Page>, p: Page): string {
  const order = GROUPS.flatMap((g) => g.pages);
  const i = order.indexOf(p.slug);
  const prev = i > 0 ? pages.get(order[i - 1]!) : undefined;
  const next = i + 1 < order.length ? pages.get(order[i + 1]!) : undefined;
  const pager = `<nav class="pager" aria-label="Pages">${prev ? `<a class="pager__prev" href="../${prev.slug}/"><span>Previous</span>${esc(prev.title)}</a>` : "<span></span>"}${next ? `<a class="pager__next" href="../${next.slug}/"><span>Next</span>${esc(next.title)}</a>` : ""}</nav>`;
  const main = `<article class="prose">
<h1>${esc(p.title)}</h1>
<p class="lead">${p.lead}</p>
${p.body}
</article>
${pager}
<p class="edit"><a class="link-quiet" href="${REPO}/edit/main/docs/guide/${p.slug}.md">Edit this page on GitHub</a></p>`;
  return shell({
    title: `${p.title} · Monita docs`,
    up: "../../",
    active: "docs",
    side: sidebar(pages, "../../", p.slug),
    main,
    toc: toc(p.headings),
    description: p.lead.replace(/<[^>]+>/g, ""),
    path: `docs/${p.slug}/`,
  });
}

function overview(pages: Map<string, Page>): string {
  const cards = GROUPS.map((g) => {
    const first = pages.get(g.pages[0]!)!;
    return `<a class="guide-card" href="${first.slug}/">
  <span class="guide-card__icon">${icon(g.icon)}</span>
  <span class="guide-card__body"><span class="guide-card__title">${g.title}</span><span class="guide-card__text">${esc(g.blurb)}</span></span>
  <span class="guide-card__foot">Start with ${esc(first.title)}${icon("arrow")}</span>
</a>`;
  }).join("\n");
  const apis: [string, string, string][] = [
    ["heartbeats", "#ping-on-success", "Ping URL: report a cron job's success or failure"],
    ["status-page", "#status-json", "Status JSON: build your own status widget"],
    ["tickets", "#issue-a-ref-id", "Ref IDs: tie tickets to your users"],
    ["tickets", "#submit-a-ticket", "Tickets: let users report bugs from your app"],
    ["incidents", "#webhook-payload", "Webhooks: route alerts to chat"],
  ];
  const api = apis.map(([slug, hash, text]) => {
    const [name, desc] = text.split(": ");
    return `<li><a href="${slug}/${hash}"><strong>${name}</strong><span>${desc}</span>${icon("arrow")}</a></li>`;
  }).join("");
  const main = `<article class="prose">
<h1>Documentation</h1>
<p class="lead">Watch every project's URLs and cron jobs, alert the right people, and tell users what is going on. Start with a monitor on your laptop, then put Monita on a server.</p>
<h2 id="guides"><a class="anchor" href="#guides">Guides</a></h2>
<div class="guide-grid">${cards}</div>
<h2 id="integrate"><a class="anchor" href="#integrate">Integrate</a></h2>
<p>Every way a project's code talks to Monita.</p>
<ul class="api-list">${api}</ul>
</article>`;
  return shell({
    title: "Monita docs",
    up: "../",
    active: "docs",
    side: sidebar(pages, "../", ""),
    main,
    toc: toc([["guides", "Guides"], ["integrate", "Integrate"]]),
    description: "Monita documentation: getting started, integration and operations.",
    path: "docs/",
  });
}

function save(file: string, body: string | Buffer): void {
  mkdirSync(dirname(file), { recursive: true });
  writeFileSync(file, body);
}

// Every code block gets highlighted tokens and a copy button. app.js answers [data-copy].
function codeBlocks(html: string): string {
  let n = 0;
  return html.replace(/<pre( class="[^"]*")?><code(?: class="language-(\w+)")?>([\s\S]*?)<\/code><\/pre>/g, (_, cls = "", lang = "", body: string) => {
    const id = `code-${++n}`;
    const code = `<code${lang ? ` class="language-${lang}"` : ""}>${highlight(body, lang)}</code>`;
    return `<div class="codeblock"><pre id="${id}"${cls}>${code}</pre><button class="codeblock__copy" type="button" data-copy="${id}">Copy</button></div>`;
  });
}

// Pages lets browsers cache every file for 10 minutes. A content hash in each asset URL stops a
// new page from loading the old CSS or JS, as platform::assets::url does for the app.
function versioned(html: string, assets: string): string {
  return html.replace(/assets\/([\w.-]+\.(?:css|js|svg|png))"/g, (_, f: string) => {
    const hash = createHash("sha256").update(readFileSync(join(assets, f))).digest("hex").slice(0, 8);
    return `assets/${f}?v=${hash}"`;
  });
}

function main(): void {
  const files = readdirSync(GUIDE).filter((f) => f.endsWith(".md")).map((f) => f.slice(0, -3));
  const listed = new Set(GROUPS.flatMap((g) => g.pages));
  const missing = files.filter((f) => !listed.has(f));
  if (missing.length > 0) throw new Error(`docs/guide pages in no sidebar group: ${missing.join(", ")}`);

  rmSync(OUT, { recursive: true, force: true });
  const assets = join(OUT, "assets");
  save(join(assets, "app.css"), readFileSync(join(ROOT, "site/app.css"), "utf8").replaceAll('url("/static/', 'url("./'));
  for (const f of readdirSync(join(ROOT, "static/fonts"))) save(join(assets, "fonts", f), readFileSync(join(ROOT, "static/fonts", f)));
  copyFileSync(join(ROOT, "site/app.js"), join(assets, "app.js"));
  for (const f of ["theme.js", "favicon.svg"]) copyFileSync(join(ROOT, "static", f), join(assets, f));
  for (const f of ["site.css", "docs.js", "landing.js"]) copyFileSync(join(import.meta.dir, f), join(assets, f));
  copyFileSync(join(ROOT, "docs/assets/logo.svg"), join(assets, "logo.svg"));
  for (const f of readdirSync(join(ROOT, "docs/assets/screens"))) copyFileSync(join(ROOT, "docs/assets/screens", f), join(assets, f));

  const page = (file: string, html: string) => save(file, versioned(codeBlocks(html), assets));
  const pages = new Map([...listed].map((slug) => [slug, load(slug)]));
  for (const p of pages.values()) page(join(OUT, "docs", p.slug, "index.html"), pageHtml(pages, p));
  page(join(OUT, "docs", "index.html"), overview(pages));
  page(join(OUT, "index.html"), readFileSync(join(import.meta.dir, "index.html"), "utf8").replace("<!-- topbar -->", topbar("", "home"))
    .replace("<!-- og -->", ogTags(
      "Monita: self-hosted uptime monitoring, heartbeats and status pages",
      "Monita checks your team's URLs and cron jobs, opens an incident after two failed checks, alerts the people you choose by email and webhook, and shows it on a public status page. You host it yourself.",
      "",
    )));
  console.log(`site: ${pages.size} guide pages in ${OUT}`);
}

if (import.meta.main) main();
