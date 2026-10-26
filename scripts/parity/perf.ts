// Times two Monita binaries on the same week of load (perf.sql on a copy of the parity fixture).
//
//   bun scripts/parity/perf.ts --base <old release binary> --new <new release binary> --fixture <dir>
//
// Run it with release builds: debug timings are several times slower and mislead. The fixture is
// the one `run.ts` leaves in <out>/fixture.

import { cpSync, mkdirSync, rmSync, writeFileSync, statSync } from "node:fs";
import { join, resolve } from "node:path";
import { parseArgs } from "node:util";

const { values: args } = parseArgs({ options: { base: { type: "string" }, new: { type: "string" }, fixture: { type: "string" }, out: { type: "string" } } });
if (!args.base || !args.new || !args.fixture) {
  console.error("usage: bun scripts/parity/perf.ts --base <bin> --new <bin> --fixture <dir> [--out dir]");
  process.exit(2);
}
const OUT = resolve(args.out ?? "target/parity-perf");
const PORT = 18191;
const URL_ = `http://127.0.0.1:${PORT}`;
const RUNS = 15;

rmSync(OUT, { recursive: true, force: true });
mkdirSync(OUT, { recursive: true });
const seed = join(OUT, "seed");
cpSync(resolve(args.fixture), seed, { recursive: true });
console.log("growing the fixture to a week of load (about a minute)");
await Bun.$`sqlite3 ${join(seed, "monita.db")} < ${new URL("./perf.sql", import.meta.url).pathname}`.quiet();
const rows = (await Bun.$`sqlite3 ${join(seed, "monita.db")} "SELECT (SELECT count(*) FROM check_results) || ' checks, ' || (SELECT count(*) FROM pings) || ' pings'"`.text()).trim();
const sql1 = async (q: string): Promise<string> => (await Bun.$`sqlite3 ${join(seed, "monita.db")} ${q}`.text()).trim();
const paymentsMonitor = await sql1("SELECT min(m.id) FROM monitors m JOIN projects p ON p.id = m.project_id WHERE p.slug = 'payments'");
const tokens = (await sql1("SELECT token FROM heartbeats WHERE token LIKE 'perftoken%' ORDER BY id LIMIT 20")).split("\n");

const READS = [
  "/api/ui/projects-chart",
  "/api/ui/projects",
  "/api/ui/projects/payments/overview",
  "/api/ui/projects/portal/overview",
  "/api/ui/projects/payments/monitors",
  "/api/ui/projects/portal/monitors",
  "/api/ui/projects/payments/heartbeats",
  `/api/ui/projects/payments/monitors/${paymentsMonitor}?range=hour`,
  `/api/ui/projects/payments/monitors/${paymentsMonitor}?range=day`,
  `/api/ui/projects/payments/monitors/${paymentsMonitor}?range=week`,
  `/api/ui/projects/payments/monitors/${paymentsMonitor}?range=month`,
  "/api/ui/incidents",
  "/api/ui/tickets",
  "/api/ui/public/portal/status",
];

const median = (xs: number[]): number => [...xs].sort((a, b) => a - b)[Math.floor(xs.length / 2)]!;

type Server = { label: string; url: string; proc: Bun.Subprocess; headers: Record<string, string>; out: Record<string, number> };

async function launch(label: string, bin: string, port: number): Promise<Server> {
  const dir = join(OUT, label);
  cpSync(seed, dir, { recursive: true });
  const url = `http://127.0.0.1:${port}`;
  const t0 = performance.now();
  const proc = Bun.spawn([resolve(bin), "serve"], {
    env: { ...process.env, MONITA_ENV: "dev", MONITA_BIND: `127.0.0.1:${port}`, MONITA_BASE_URL: url, MONITA_DB_PATH: join(dir, "monita.db"), MONITA_LOG: "warn" },
    stdout: "ignore",
    stderr: Bun.file(join(dir, "server.log")),
  });
  while (!(await fetch(`${url}/healthz`).then((r) => r.ok).catch(() => false))) await Bun.sleep(5);
  const started = performance.now() - t0;
  const login = await fetch(`${url}/api/ui/login`, { method: "POST", headers: { "content-type": "application/json", origin: url }, body: JSON.stringify({ email: "admin@monita.test", password: "monita-dev-password" }) });
  const cookie = login.headers.getSetCookie().map((c) => c.split(";")[0]).join("; ");
  return { label, url, proc, headers: { cookie, "x-utc-offset": "420", "accept-encoding": "identity" }, out: { "start to /healthz": started } };
}

/** Every sample asks base, new and base again in turn, the order rotating, so a busy machine
 * slows all three alike. */
async function measure(servers: Server[]): Promise<void> {
  for (const path of READS) {
    for (const s of servers) await (await fetch(s.url + path, { headers: s.headers })).arrayBuffer();
    const times: number[][] = servers.map(() => []);
    for (let i = 0; i < RUNS; i++) {
      for (let k = 0; k < servers.length; k++) {
        const j = (i + k) % servers.length;
        const s = servers[j]!;
        const t = performance.now();
        const res = await fetch(s.url + path, { headers: s.headers });
        await res.arrayBuffer();
        if (!res.ok) throw new Error(`${s.label} ${path}: ${res.status}`);
        times[j]!.push(performance.now() - t);
      }
    }
    servers.forEach((s, j) => (s.out[path] = median(times[j]!)));
  }
  const pings: number[][] = servers.map(() => []);
  for (const token of tokens) {
    for (const [j, s] of servers.entries()) {
      const t = performance.now();
      await (await fetch(`${s.url}/ping/${token}`)).arrayBuffer();
      pings[j]!.push(performance.now() - t);
    }
  }
  servers.forEach((s, j) => (s.out["a heartbeat ping (write)"] = median(pings[j]!)));
  for (const s of servers) s.out["server memory after (MiB)"] = Number((await Bun.$`ps -o rss= -p ${s.proc.pid}`.text()).trim()) / 1024;
}

// Base, new, base again: the spread between the two base runs is the noise. A measure is slower
// only when the new run is above both base runs by more than 20% and 1 ms.
const servers = [await launch("base", args.base, PORT), await launch("new", args.new, PORT + 1), await launch("base-2", args.base, PORT + 2)];
try {
  await measure(servers);
} finally {
  for (const s of servers) {
    s.proc.kill("SIGTERM");
    await s.proc.exited;
  }
}
const [base, next, again] = servers.map((s) => s.out) as [Record<string, number>, Record<string, number>, Record<string, number>];
const size = (p: string): string => `${(statSync(resolve(p)).size / 1e6).toFixed(1)} MB`;
let slower = 0;
const lines = [
  `# Performance: ${rows}, release builds, median of ${RUNS}`,
  ``,
  `| Measure | Base | New | Base again | Verdict |`,
  `| --- | --- | --- | --- | --- |`,
  `| Binary size | ${size(args.base)} | ${size(args.new)} | | |`,
  ...Object.keys(base).map((k) => {
    const unit = k.includes("MiB") ? "" : " ms";
    const hi = Math.max(base[k]!, again[k]!);
    const lo = Math.min(base[k]!, again[k]!);
    const n = next[k]!;
    const verdict = n > hi * 1.2 && n - hi > 1 ? (slower++, "**slower**") : n < lo * 0.8 && lo - n > 1 ? "faster" : "same";
    return `| ${k} | ${base[k]!.toFixed(1)}${unit} | ${n.toFixed(1)}${unit} | ${again[k]!.toFixed(1)}${unit} | ${verdict} |`;
  }),
];
writeFileSync(join(OUT, "perf.md"), lines.join("\n"));
console.log(lines.join("\n"));
process.exit(slower ? 1 : 0);
