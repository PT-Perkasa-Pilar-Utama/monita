import { availableParallelism, loadavg, totalmem } from "node:os";
import { defineConfig } from "@playwright/test";
import { describeCapacity, workersFor } from "./lib/capacity";
import { BASE_URL, PORT, RESET_TOKEN } from "./lib/env";
import { projectsFor } from "./lib/shards";

const ci = process.env["CI"] !== undefined;
const capacity = {
  cpus: availableParallelism(),
  load1: loadavg()[0] ?? 0,
  totalGiB: totalmem() / 1024 ** 3,
  ci,
  override: process.env["E2E_WORKERS"],
};
const workers = workersFor(capacity);
console.log(describeCapacity(capacity, workers));

export default defineConfig({
  testDir: "./specs",
  workers,
  retries: 0,
  forbidOnly: ci,
  grepInvert: process.env["E2E_FULL"] ? undefined : /@heavy/,
  reporter: [["list"], ["html", { open: "never" }], ["./lib/timing.ts"]],
  globalSetup: "./global-setup.ts",
  use: {
    baseURL: BASE_URL,
    channel: "chrome",
    headless: true,
    trace: "retain-on-failure",
    screenshot: "only-on-failure",
  },
  projects: projectsFor(process.env["E2E_SHARD"]),
  // Playwright starts the debug binary and stops it at the end. `make e2e` builds it first.
  webServer: {
    command: "../target/debug/monita serve",
    url: `${BASE_URL}/healthz`,
    reuseExistingServer: !ci,
    timeout: 30_000,
    env: {
      MONITA_ENV: "test",
      MONITA_BIND: `127.0.0.1:${PORT}`,
      MONITA_BASE_URL: BASE_URL,
      MONITA_DB_PATH: "../data/e2e.db",
      MONITA_RESET_TOKEN: RESET_TOKEN,
      MONITA_LOG: "warn",
    },
  },
});
