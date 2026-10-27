/**
 * Pure helpers for the online demo. demo/snapshot.ts records the app's API answers on the demo
 * seed; demo/sw.ts, a service worker, answers the running app from that recording.
 */

/** The origin the recording server runs as. Ping URLs and links on the pages show it. */
export const DEMO_ORIGIN = "https://monita.example.com";

/** One recorded answer. A JSON or text body is kept as text, any other body as base64. */
export type Entry = { status: number; type: string; body: string; base64?: true };

export type Recording = { capturedAt: number; entries: Record<string, Entry> };

const ATTR = /\b(href|src|action|xlink:href)="(?:https:\/\/monita\.example\.com)?(\/[^"]*)"/g;

/** The site path of a page: "/projects/portal/monitors" is saved as its folder's index.html. */
export function fileFor(path: string): string {
  const clean = path.replace(/^\/+|\/+$/g, "");
  return clean === "" ? "index.html" : `${clean}/index.html`;
}

/** Same-site links in rendered `html` the crawler may follow: hrefs with no query that are pages. */
export function pageLinks(html: string): string[] {
  const out = new Set<string>();
  for (const [, attr, url] of html.matchAll(ATTR)) {
    const path = url!.replace(/[?#].*$/, "") || "/";
    const query = /\?/.test(url!.replace(/#.*$/, ""));
    if (attr === "href" && !query && !path.startsWith("/static/") && !path.startsWith("/assets/")) out.add(path.replace(/\/+$/, "") || "/");
  }
  return [...out];
}

/** A request the demo answers from the recording: the UI API, project icons and user pictures. */
export function recordable(path: string): boolean {
  return path.startsWith("/api/ui/") || /^\/projects\/[^/]+\/icon$/.test(path) || /^\/users\/\d+\/avatar$/.test(path);
}

// Unix seconds from 2001 to 2096: an id or a count never reaches them in the demo seed.
const isTime = (n: number): boolean => Number.isInteger(n) && n >= 1e9 && n < 4e9;
const TIME_KEY = /(^|_)(at|last|from|to|since|until)$/;

/**
 * `value` with every time moved `delta` seconds later: a Unix time under a key such as
 * `checked_at`, `last` or `prev_to`, and each `YYYY-MM-DD` in a string, by whole days.
 */
export function shiftTimes(value: unknown, delta: number, key = ""): unknown {
  if (Array.isArray(value)) return value.map((v) => shiftTimes(v, delta, key));
  if (value !== null && typeof value === "object") {
    return Object.fromEntries(Object.entries(value).map(([k, v]) => [k, shiftTimes(v, delta, k)]));
  }
  if (typeof value === "number" && TIME_KEY.test(key) && isTime(value)) return value + delta;
  if (typeof value === "string") {
    const days = Math.round(delta / 86_400);
    return value.replace(/\b(\d{4})-(\d{2})-(\d{2})\b/g, (d) => new Date(Date.parse(`${d}T00:00:00Z`) + days * 86_400_000).toISOString().slice(0, 10));
  }
  return value;
}

/** A query with each time in it moved back `delta` seconds, so a cursor from a moved answer finds its recording. */
function unshift(search: string, delta: number): string {
  return search.replace(/\b\d{10}\b/g, (n) => (isTime(Number(n)) ? String(Number(n) - delta) : n));
}

/** The query's names and values, with each time as a number for a near match. */
function params(search: string): Map<string, string> {
  return new Map(new URLSearchParams(search));
}

/**
 * The recorded key that answers `path` + `search`: the same query, else the same query with
 * times up to 10 minutes off (the shift moves while a page is open), else the path alone.
 */
export function lookup(keys: Iterable<string>, path: string, search: string, delta: number): string | null {
  const all = new Set(keys);
  const back = unshift(search, delta);
  if (all.has(path + back)) return path + back;
  const want = params(back);
  for (const key of all) {
    const [p, q = ""] = key.split(/(?=\?)/, 2) as [string, string?];
    if (p !== path || q === "") continue;
    const got = params(q);
    if (got.size !== want.size) continue;
    const near = [...want].every(([k, v]) => {
      const g = got.get(k);
      if (g === v) return true;
      return g !== undefined && isTime(Number(v)) && isTime(Number(g)) && Math.abs(Number(v) - Number(g)) <= 600;
    });
    if (near) return key;
  }
  return all.has(path) ? path : null;
}
