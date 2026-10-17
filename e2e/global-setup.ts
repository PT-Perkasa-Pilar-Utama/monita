import { mkdirSync, writeFileSync } from "node:fs";
import { BASE_URL, RESET_TOKEN } from "./lib/env";
import { AUTH_DIR, ROLES, SEED_PASSWORD } from "./lib/roles";

/** Resets to the QA seed and signs each role in once. Never reset inside playwright.config.ts. */
export default async function globalSetup(): Promise<void> {
  const res = await fetch(`${BASE_URL}/admin/reset-state`, {
    method: "POST",
    headers: { "content-type": "application/json", "x-reset-token": RESET_TOKEN },
    body: JSON.stringify({ seed: "qa" }),
  });
  if (res.status !== 204) throw new Error(`reset-state returned ${res.status}: ${await res.text()}`);

  mkdirSync(AUTH_DIR, { recursive: true });
  for (const [role, email] of Object.entries(ROLES)) {
    const login = await fetch(`${BASE_URL}/api/ui/login`, {
      method: "POST",
      headers: { "content-type": "application/json", origin: BASE_URL },
      body: JSON.stringify({ email, password: SEED_PASSWORD }),
    });
    const cookie = login.headers.getSetCookie().find((c) => c.startsWith("monita_session="));
    if (login.status !== 204 || cookie === undefined) throw new Error(`sign-in for ${role} returned ${login.status}`);
    const value = cookie.split(";")[0]!.slice("monita_session=".length);
    const state = {
      cookies: [{ name: "monita_session", value, domain: "127.0.0.1", path: "/", expires: -1, httpOnly: true, secure: false, sameSite: "Lax" }],
      origins: [],
    };
    writeFileSync(`${AUTH_DIR}/${role}.json`, JSON.stringify(state));
  }
}
