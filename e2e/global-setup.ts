import { BASE_URL, RESET_TOKEN } from "./lib/env";

/** Resets the database to the QA seed once per run. Never reset inside playwright.config.ts. */
export default async function globalSetup(): Promise<void> {
  const res = await fetch(`${BASE_URL}/admin/reset-state`, {
    method: "POST",
    headers: { "content-type": "application/json", "x-reset-token": RESET_TOKEN },
    body: JSON.stringify({ seed: "qa" }),
  });
  if (res.status !== 204) throw new Error(`reset-state returned ${res.status}: ${await res.text()}`);
  // ponytail: sign-in per role and storageState arrive with US-03; add one file per role then.
}
