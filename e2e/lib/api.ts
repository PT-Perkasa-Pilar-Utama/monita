import { expect, type Page } from "@playwright/test";
import { BASE_URL } from "./env";

/** Writes as the browser's fetch sends them: JSON with Monita's origin, with the page's cookie. */
const HEADERS = { origin: BASE_URL };

/** POST, PUT or DELETE `path` and expect `status`; returns the `data` of the answer. */
export async function apiSend<T = unknown>(page: Page, method: "POST" | "PUT" | "DELETE", path: string, data: object, status: number): Promise<T> {
  const res = await page.request.fetch(path, { method, headers: HEADERS, data });
  expect(res.status(), `${method} ${path}: ${await res.text()}`).toBe(status);
  return res.status() === 204 ? (undefined as T) : ((await res.json()) as { data: T }).data;
}

export async function apiGet<T = unknown>(page: Page, path: string): Promise<T> {
  const res = await page.request.get(path);
  expect(res.status(), `GET ${path}`).toBe(200);
  return ((await res.json()) as { data: T }).data;
}

/** A project through the API, for tests about something else. Returns its slug. */
export async function apiProject(page: Page, name: string): Promise<string> {
  return (await apiSend<{ slug: string }>(page, "POST", "/api/ui/projects", { name, homepage_url: "https://app.example.com" }, 201)).slug;
}

/** A monitor on an address nothing answers, so it never passes by chance. Returns its id. */
export async function apiMonitor(page: Page, slug: string, fields: object = {}): Promise<number> {
  const form = { name: "API", url: "http://127.0.0.1:9/", interval_s: "60", timeout_s: "5", expected_status: "2xx", severity: "warning", is_public: true, method: "GET", ...fields };
  return (await apiSend<{ id: number }>(page, "POST", `/api/ui/projects/${slug}/monitors`, form, 201)).id;
}

/** A public heartbeat. Returns its id and the URL a job calls. */
export async function apiHeartbeat(page: Page, slug: string, name = "nightly-settlement"): Promise<{ id: number; pingUrl: string }> {
  const { id } = await apiSend<{ id: number }>(page, "POST", `/api/ui/projects/${slug}/heartbeats`, { name, period_s: "3600", grace_s: "300", severity: "critical", is_public: true }, 201);
  const detail = await apiGet<{ ping_url: string }>(page, `/api/ui/projects/${slug}/heartbeats/${id}`);
  return { id, pingUrl: detail.ping_url };
}
