/**
 * The demo's service worker. It answers the app's API, icon and picture requests from the
 * recording that snapshot.ts made, with every time moved forward so the data reads as live.
 * A write gets a read-only error. A page that GitHub Pages does not have gets the app's shell.
 * snapshot.ts bundles this file to sw.js at the demo's root and defines DEMO_BASE and DEMO_API.
 */
import { lookup, recordable, shiftTimes, type Recording } from "../lib/demo-site";

declare const DEMO_BASE: string;
declare const DEMO_API: string;

type Client = { postMessage(message: unknown): void };
type FetchEvent = { request: Request; respondWith(answer: Promise<Response>): void };
type Scope = {
  addEventListener(type: "install" | "activate" | "message", fn: (e: { waitUntil(p: Promise<unknown>): void }) => void): void;
  addEventListener(type: "fetch", fn: (e: FetchEvent) => void): void;
  skipWaiting(): Promise<void>;
  clients: { claim(): Promise<void>; matchAll(): Promise<Client[]> };
  location: Location;
};
const sw = self as unknown as Scope;

const READ_ONLY = "This is a read-only demo. Install Monita to try this.";

let recording: Promise<Recording> | null = null;
const load = (): Promise<Recording> => {
  if (recording === null) {
    recording = fetch(`${DEMO_BASE}/_demo/${DEMO_API}`).then((r) => {
      if (!r.ok) throw new Error(`the recording answered ${r.status}`);
      return r.json() as Promise<Recording>;
    });
    // A failed load is tried again on the next request.
    recording.catch(() => (recording = null));
  }
  return recording;
};

const json = (status: number, body: unknown): Response =>
  new Response(JSON.stringify(body), { status, headers: { "content-type": "application/json", "cache-control": "no-store" } });

async function answer(request: Request, url: URL): Promise<Response> {
  const path = url.pathname;
  if (request.method !== "GET" && request.method !== "HEAD") {
    // Signing out and in again works, so a visitor can try the sign-in page and come back.
    if (path === "/api/ui/logout" || path === "/api/ui/login") return new Response(null, { status: 204 });
    for (const c of await sw.clients.matchAll()) c.postMessage({ demo: "read-only" });
    return json(403, { error: { code: "demo", message: READ_ONLY } });
  }
  const data = await load();
  const delta = Math.floor(Date.now() / 1000) - data.capturedAt;
  const key = lookup(Object.keys(data.entries), path, url.search, delta);
  if (key === null) return json(404, { error: { code: "not_found", message: "The demo has no recording of this page." } });
  const e = data.entries[key]!;
  const headers = { "content-type": e.type, "cache-control": "no-store" };
  if (e.base64) return new Response(Uint8Array.from(atob(e.body), (c) => c.charCodeAt(0)), { status: e.status, headers });
  if (e.type.includes("json")) return new Response(JSON.stringify(shiftTimes(JSON.parse(e.body), delta)), { status: e.status, headers });
  return new Response(e.body, { status: e.status, headers });
}

// A page path the site has no file for, such as a list with a new filter: the app's shell.
async function page(request: Request, url: URL): Promise<Response> {
  const res = await fetch(request);
  if (res.status !== 404) return res;
  const shell = url.pathname.startsWith(`${DEMO_BASE}/status/`) ? "status.html" : "index.html";
  const body = await fetch(`${DEMO_BASE}/${shell}`);
  return new Response(body.body, { status: 200, headers: { "content-type": "text/html; charset=utf-8" } });
}

sw.addEventListener("install", (e) => e.waitUntil(sw.skipWaiting()));
sw.addEventListener("activate", (e) => e.waitUntil(sw.clients.claim()));
// boot.js asks for this after a hard reload, which loads a page outside the worker.
sw.addEventListener("message", (e) => e.waitUntil(sw.clients.claim()));
sw.addEventListener("fetch", (e) => {
  const url = new URL(e.request.url);
  if (url.origin !== sw.location.origin) return;
  if (recordable(url.pathname)) e.respondWith(answer(e.request, url));
  else if (e.request.mode === "navigate" && url.pathname.startsWith(`${DEMO_BASE}/`)) e.respondWith(page(e.request, url));
});
