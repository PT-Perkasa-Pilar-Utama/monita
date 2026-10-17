/** Pure helpers for the static demo (demo/snapshot.ts): where a page is saved and how its links read. */

/** The origin the snapshot server runs as. Ping URLs and links on the pages show it. */
export const DEMO_ORIGIN = "https://monita.example.com";

/** Where a link goes that the demo cannot follow: a form, a filtered list, a page not crawled. */
export const OFF = "#demo-off";

const ATTR = /\b(href|src|action|xlink:href)="(?:https:\/\/monita\.example\.com)?(\/[^"]*)"/g;

/** The site path of a page: "/projects/portal/monitors" is saved as its folder's index.html. */
export function fileFor(path: string): string {
  const clean = path.replace(/^\/+|\/+$/g, "");
  return clean === "" ? "index.html" : `${clean}/index.html`;
}

function segments(path: string): string[] {
  return path.split("/").filter(Boolean);
}

/** A relative link from the page at `from` to the site path `to`, so the demo works under any base. */
export function relative(from: string, to: string, page: boolean): string {
  const up = "../".repeat(segments(from).length);
  const rest = segments(to).join("/");
  const link = `${up}${rest}${page && rest !== "" ? "/" : ""}`;
  return link === "" ? "./" : link;
}

function split(url: string): { path: string; query: string; hash: string } {
  const m = /^([^?#]*)(\?[^#]*)?(#.*)?$/.exec(url)!;
  return { path: m[1] || "/", query: m[2] ?? "", hash: m[3] ?? "" };
}

/** A file the page loads rather than a page: the app's built files, /static, or any `src`. */
function isAsset(attr: string, path: string): boolean {
  return attr === "src" || path.startsWith("/static/") || path.startsWith("/assets/");
}

/** Same-site links in `html` the crawler may follow: GET hrefs with no query that are pages. */
export function pageLinks(html: string): string[] {
  const out = new Set<string>();
  for (const [, attr, url] of html.matchAll(ATTR)) {
    const { path, query } = split(url!);
    if (attr === "href" && query === "" && !isAsset(attr, path)) out.add(path.replace(/\/+$/, "") || "/");
  }
  return [...out];
}

/** The files `html` loads, without their cache-busting query. */
export function assetLinks(html: string): string[] {
  const out = new Set<string>();
  for (const [, attr, url] of html.matchAll(ATTR)) {
    const { path } = split(url!);
    if (isAsset(attr!, path)) out.add(path);
  }
  return [...out];
}

/** `html` from the page at `from` with every same-site link relative, and the rest pointed at OFF. */
export function rewrite(html: string, from: string, pages: Set<string>): string {
  return html.replace(ATTR, (_, attr: string, url: string) => {
    const { path, query, hash } = split(url);
    const page = path.replace(/\/+$/, "") || "/";
    let to: string;
    if (isAsset(attr, path)) to = relative(from, path, false) + query + hash;
    else if (attr !== "action" && query === "" && pages.has(page)) to = relative(from, page, true) + hash;
    else to = OFF;
    return `${attr}="${to}"`;
  });
}

/** A built stylesheet under /assets/ with its /static font URLs made relative to it. */
export function relativeFonts(css: string): string {
  return css.replace(/url\(("?)\/static\//g, "url($1../static/");
}
