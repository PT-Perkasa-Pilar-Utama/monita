// Parity kit: proves that two Monita binaries give the same answers on the same database.
//
//   bun scripts/parity/run.ts --base <old binary> --new <new binary> [--out <dir>]
//
// 1. The base binary builds a fixture: the demo seed, a read key and a write key, every check
//    and deadline pushed far out so nothing runs on its own.
// 2. Three runs, each on its own copy of the fixture: base, new, base again. Each run reads every
//    GET route as six callers, plays a write scenario that touches every module, reads again,
//    stops, and dumps every table.
// 3. A value that differs between the two base runs is noise (a clock, a random token). A value
//    where the new run differs from both base runs is a regression. The report lists each one.
// See README.md beside this file.

import { createHmac, createHash } from "node:crypto";
import { cpSync, mkdirSync, rmSync, writeFileSync, readFileSync, existsSync } from "node:fs";
import { join, resolve } from "node:path";
import { parseArgs } from "node:util";
import { routes } from "./routes";

const { values: args } = parseArgs({
  options: { base: { type: "string" }, new: { type: "string" }, out: { type: "string" }, port: { type: "string" }, "compare-only": { type: "boolean" } },
});
if (!args["compare-only"] && (!args.base || !args.new)) {
  console.error("usage: bun scripts/parity/run.ts --base <old binary> --new <new binary> [--out dir] [--port 18190]");
  process.exit(2);
}
const BASE_BIN = resolve(args.base ?? "(from disk)");
const NEW_BIN = resolve(args.new ?? "(from disk)");
const OUT = resolve(args.out ?? "target/parity");
const PORT = Number(args.port ?? 18190);
const URL_ = `http://127.0.0.1:${PORT}`;
const PASSWORD = "monita-dev-password";
const SLUG = "portal";

type Json = unknown;
type Capture = Record<string, { status: number; body: Json }>;
type Ids = Record<string, string | number>;

const sql = async (db: string, q: string): Promise<string> => (await Bun.$`sqlite3 ${db} ${q}`.text()).trim();

async function start(bin: string, db: string): Promise<Bun.Subprocess> {
  const proc = Bun.spawn([bin, "serve"], {
    env: {
      ...process.env,
      MONITA_ENV: "dev",
      MONITA_BIND: `127.0.0.1:${PORT}`,
      MONITA_BASE_URL: URL_,
      MONITA_DB_PATH: db,
      MONITA_RESET_TOKEN: "parity-kit",
      MONITA_LOG: "warn",
    },
    stdout: "ignore",
    stderr: Bun.file(`${db}.log`),
  });
  for (let i = 0; i < 300; i++) {
    if (await fetch(`${URL_}/healthz`).then((r) => r.ok).catch(() => false)) return proc;
    if (proc.exitCode !== null) throw new Error(`${bin} exited with ${proc.exitCode}; see ${db}.log`);
    await Bun.sleep(100);
  }
  proc.kill();
  throw new Error(`${bin} did not answer /healthz in 30 s; see ${db}.log`);
}

async function stop(proc: Bun.Subprocess): Promise<void> {
  proc.kill("SIGTERM");
  await proc.exited;
}

async function login(email: string): Promise<string> {
  const res = await fetch(`${URL_}/api/ui/login`, {
    method: "POST",
    headers: { "content-type": "application/json", origin: URL_ },
    body: JSON.stringify({ email, password: PASSWORD }),
  });
  if (res.status !== 204) throw new Error(`login ${email}: ${res.status} ${await res.text()}`);
  return res.headers.getSetCookie().map((c) => c.split(";")[0]).join("; ");
}

type Who = { name: string; headers: Record<string, string> };

async function call(who: Who, method: string, path: string, body?: Json): Promise<{ status: number; body: Json }> {
  const headers: Record<string, string> = { origin: URL_, "x-utc-offset": "420", "accept-encoding": "identity", ...who.headers };
  if (body !== undefined) headers["content-type"] = "application/json";
  const res = await fetch(URL_ + path, { method, headers, body: body === undefined ? undefined : JSON.stringify(body), redirect: "manual" });
  const type = res.headers.get("content-type") ?? "";
  const raw = new Uint8Array(await res.arrayBuffer());
  let parsed: Json;
  if (type.startsWith("application/json")) parsed = raw.length ? JSON.parse(new TextDecoder().decode(raw)) : null;
  else parsed = { type, sha256: createHash("sha256").update(raw).digest("hex"), bytes: raw.length };
  return { status: res.status, body: parsed };
}

// ---------------------------------------------------------------- fixture

async function fixture(dir: string): Promise<Ids> {
  rmSync(dir, { recursive: true, force: true });
  mkdirSync(dir, { recursive: true });
  const db = join(dir, "monita.db");
  const proc = await start(BASE_BIN, db);
  try {
    const seeded = await fetch(`${URL_}/admin/reset-state`, {
      method: "POST",
      headers: { "x-reset-token": "parity-kit", "content-type": "application/json" },
      body: JSON.stringify({ seed: "demo" }),
    });
    if (seeded.status !== 204) throw new Error(`reset-state: ${seeded.status}`);
    const tl: Who = { name: "tl", headers: { cookie: await login("tl@monita.test") } };
    const key = async (scope: string): Promise<string> => {
      const r = await call(tl, "POST", `/api/ui/projects/${SLUG}/keys`, { name: `parity ${scope}`, scope });
      const k = (r.body as { data?: { key?: string } }).data?.key;
      if (!k) throw new Error(`key ${scope}: ${r.status} ${JSON.stringify(r.body)}`);
      return k;
    };
    const ids: Ids = { read_key: await key("read"), write_key: await key("write") };
    const secret = await call(tl, "POST", `/api/ui/projects/${SLUG}/secrets`, { name: "PARITY_SECRET", value: "parity" });
    if (secret.status !== 201) throw new Error(`secret: ${secret.status} ${JSON.stringify(secret.body)}`);
    await stop(proc);
    // Nothing runs on its own during a capture: checks, deadlines and outbox retries move out.
    await sql(db, "UPDATE monitors SET next_check_at = 4000000000; UPDATE heartbeats SET deadline_at = 4000000000 WHERE deadline_at IS NOT NULL; UPDATE monitor_locations SET next_check_at = 4000000000; UPDATE outbox SET next_attempt_at = 4000000000 WHERE sent_at IS NULL AND failed_at IS NULL;");
    const one = async (q: string): Promise<string> => {
      const v = await sql(db, q);
      if (!v) throw new Error(`fixture has no row for: ${q}`);
      return v;
    };
    const p = `(SELECT id FROM projects WHERE slug = '${SLUG}')`;
    Object.assign(ids, {
      monitor: await one(`SELECT min(id) FROM monitors WHERE project_id = ${p}`),
      heartbeat: await one(`SELECT min(id) FROM heartbeats WHERE project_id = ${p}`),
      incident: await one(`SELECT i.id FROM incidents i JOIN status_updates u ON u.incident_id = i.id WHERE i.project_id = ${p} ORDER BY i.id LIMIT 1`),
      maintenance: await one(`SELECT min(id) FROM maintenance_windows WHERE project_id = ${p}`),
      changelog: await one(`SELECT min(id) FROM changelog_entries WHERE project_id = ${p}`),
      version: await one(`SELECT version FROM changelog_entries WHERE project_id = ${p} ORDER BY id LIMIT 1`),
      card: await one(`SELECT min(id) FROM alert_cards WHERE project_id = ${p}`),
      secret: await one(`SELECT min(id) FROM project_secrets WHERE project_id = ${p}`),
      key: await one(`SELECT min(id) FROM api_keys WHERE project_id = ${p}`),
      ticket: await one(`SELECT min(number) FROM tickets WHERE project_id = ${p}`),
      location: await one("SELECT min(id) FROM locations"),
      user: await one("SELECT id FROM users WHERE email = 'tl@monita.test'"),
    });
    writeFileSync(join(dir, "ids.json"), JSON.stringify(ids, null, 2));
    return ids;
  } finally {
    if (proc.exitCode === null) await stop(proc);
  }
}

// ---------------------------------------------------------------- reads

const NOT_READS = [/^\/ping\//, /^\/api\/v1\/probe\//, /^\/api\/ui\/invites\//, /^\/api\/v1\/tickets\/mine/];

function fill(path: string, ids: Ids, assets: { js: string }): string | null {
  const id: Record<string, string> = {
    monitors: "monitor", heartbeats: "heartbeat", incidents: "incident", maintenance: "maintenance", changelog: "changelog",
    "alert-cards": "card", secrets: "secret", keys: "key", locations: "location", users: "user",
  };
  const parts = path.split("/");
  for (let i = 0; i < parts.length; i++) {
    const s = parts[i]!;
    if (s === "{slug}") parts[i] = SLUG;
    else if (s === "{number}") parts[i] = String(ids.ticket);
    else if (s === "{version}") parts[i] = String(ids.version);
    else if (s === "{id}") {
      const k = id[parts[i - 1]!];
      if (!k) return null;
      parts[i] = String(ids[k]);
    } else if (s === "{*path}") parts[i] = path.startsWith("/static/") ? "favicon.svg" : assets.js;
    else if (s.startsWith("{")) return null;
  }
  return parts.join("/").replace(/\/$/, "") || "/";
}

function variants(path: string): string[] {
  if (/\/(monitors|heartbeats)\/\d+$/.test(path)) return ["", "?range=hour", "?range=day", "?range=week", "?range=month"];
  if (/\/incidents$/.test(path)) return ["", "?state=resolved", "?state=all"];
  if (/\/maintenance$/.test(path)) return ["", "?state=ended"];
  if (path === "/api/ui/projects") return ["", "?page=2", "?q=pay"];
  if (/\/tickets$/.test(path)) return ["", "?stage=done"];
  if (/\/public\/[^/]+\/(history|incidents|changelog)$/.test(path)) return ["", "?page=2"];
  return [""];
}

async function reads(cap: Capture, tag: string, people: Who[], ids: Ids, assets: { js: string }): Promise<void> {
  for (const r of await routes()) {
    if (!r.methods.includes("GET") || NOT_READS.some((x) => x.test(r.path))) continue;
    const path = fill(r.path, ids, assets);
    if (!path) {
      cap[`SKIP ${r.path}`] = { status: 0, body: "no value for a path parameter" };
      continue;
    }
    for (const v of variants(path)) {
      for (const who of people) {
        cap[`${tag} GET ${who.name} ${path}${v}`] = await call(who, "GET", path + v);
      }
    }
  }
}

// ---------------------------------------------------------------- writes

function totp(secret: string, at = Date.now()): string {
  const alphabet = "ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";
  let bits = "";
  for (const ch of secret.replace(/=+$/, "").toUpperCase()) bits += alphabet.indexOf(ch).toString(2).padStart(5, "0");
  const key = Buffer.from(bits.match(/.{8}/g)!.map((b) => parseInt(b, 2)));
  const counter = Buffer.alloc(8);
  counter.writeBigUInt64BE(BigInt(Math.floor(at / 1000 / 30)));
  const h = createHmac("sha1", key).update(counter).digest();
  const o = h[h.length - 1]! & 0xf;
  return String((h.readUInt32BE(o) & 0x7fffffff) % 1_000_000).padStart(6, "0");
}

// A 2x2 PNG, so the avatar and icon paths store and serve a real blob.
const PNG = "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAIAAAACCAIAAAD91JpzAAAAFklEQVR4nGP8z8DAwMDAxMDAwMDAAAANHQEDasKb6QAAAABJRU5ErkJggg==";

async function writes(cap: Capture, p: Record<string, Who>, ids: Ids): Promise<void> {
  let n = 0;
  // The name holds the route, not the path: a path can carry a random token.
  const step = async (who: Who, method: string, path: string, body?: Json, route = path): Promise<Json> => {
    const r = await call(who, method, path, body);
    cap[`W${String(++n).padStart(3, "0")} ${method} ${who.name} ${route}`] = r;
    // A write that makes a check or a ping due waits for the 1 s scheduler tick, so each run
    // stores its check rows at the same point of the scenario.
    if (/\/(monitors|heartbeats|incidents|maintenance|ping)\b/.test(route) && method !== "GET") await Bun.sleep(1300);
    return r.body;
  };
  const idOf = (b: Json): number => (b as { data?: { id?: number } }).data?.id ?? -1;
  const { admin, tl, write } = p as Record<string, Who>;
  const t = Math.floor(Date.now() / 1000);
  const ui = `/api/ui/projects/${SLUG}`;
  const v1 = `/api/v1/projects/${SLUG}`;

  // Projects and their settings.
  const made = (await step(admin!, "POST", "/api/ui/projects", { name: "Parity Kit", homepage_url: "https://kit.example.com" })) as { data?: { slug?: string } };
  await step(tl!, "PUT", `${ui}/profile`, { name: "Customer Portal", description: "Edited by the kit.", homepage_url: "https://portal.example.com", icon: PNG });
  await step(tl!, "PUT", `${ui}/public-pages`, { incidents: true, known_issues: true, report: true, changelog: true });
  await step(tl!, "PUT", `${ui}/allowlist`, { cidrs: "10.0.0.0/8" });
  await step(tl!, "PUT", `${ui}/allowlist`, { cidrs: "" });

  // Monitors and heartbeats, by the team and by a write key.
  const http = idOf(await step(tl!, "POST", `${ui}/monitors`, { name: "Kit HTTP", url: "http://127.0.0.1:1/", interval_s: "300", timeout_s: "10", expected_status: "2xx", severity: "warning" }));
  const tcp = idOf(await step(write!, "POST", `${v1}/monitors`, { kind: "tcp", name: "Kit TCP", url: "127.0.0.1:1", interval_s: "300", timeout_s: "10", expected_status: "2xx", severity: "critical" }));
  await step(tl!, "PUT", `${ui}/monitors/${http}`, { name: "Kit HTTP edited", url: "http://127.0.0.1:1/health", interval_s: "180", timeout_s: "5", expected_status: "200", severity: "critical" });
  await step(tl!, "PUT", `${ui}/monitors/${http}/paused`, { paused: true });
  await step(tl!, "PUT", `${ui}/monitors/${http}/paused`, { paused: false });
  await step(write!, "POST", `${v1}/monitors/batch`, { action: "pause", ids: [http, tcp] });
  await step(write!, "POST", `${v1}/monitors/batch`, { action: "resume", ids: [http, tcp] });
  const hb = idOf(await step(tl!, "POST", `${ui}/heartbeats`, { name: "Kit job", period_s: "3600", grace_s: "300", severity: "warning" }));
  const hbView = (await step(tl!, "GET", `${ui}/heartbeats/${hb}`)) as { data?: { ping_url?: string } };
  const ping = hbView.data?.ping_url ? new URL(hbView.data.ping_url).pathname : null;
  if (ping) {
    await step(p.anon!, "GET", ping, undefined, "/ping/{token}");
    await Bun.sleep(1100);
    await step(p.anon!, "POST", `${ping}/fail`, undefined, "/ping/{token}/fail");
  }
  await step(tl!, "PUT", `${ui}/heartbeats/${hb}/paused`, { paused: true });
  await step(tl!, "PUT", `${ui}/heartbeats/${hb}`, { name: "Kit job edited", period_s: "86400", grace_s: "1800", severity: "critical" });

  // Incidents: manual, updates, reclassify, resolve.
  const inc = idOf(await step(tl!, "POST", `${ui}/incidents`, { title: "Kit outage", items: [`monitor:${ids.monitor}`], severity: "critical", message: "Opened by the kit." }));
  await step(write!, "POST", `${v1}/incidents/${inc}/updates`, { label: "identified", message: "Found by a key." });
  await step(tl!, "PUT", `${ui}/incidents/${inc}/class`, { class: "planned" });
  await step(tl!, "PUT", `${ui}/incidents/${inc}/class`, { class: "counted" });
  await step(tl!, "POST", `${ui}/incidents/${inc}/updates`, { label: "resolved", message: "Fixed by the kit." });

  // Maintenance: one now, one later.
  const now = idOf(await step(tl!, "POST", `${ui}/maintenance`, { title: "Kit window now", starts_at: t - 60, ends_at: t + 3600, items: [`monitor:${http}`] }));
  const later = idOf(await step(tl!, "POST", `${ui}/maintenance`, { title: "Kit window later", starts_at: t + 7200, ends_at: t + 9000, items: [`heartbeat:${hb}`] }));
  await step(tl!, "POST", `${ui}/maintenance/${now}/end`, {});
  await step(tl!, "DELETE", `${ui}/maintenance/${later}`);

  // Changelog, keys, secrets, alert cards.
  const ch = idOf(await step(tl!, "POST", `${ui}/changelog`, { version: "9.9.9", released_on: "2026-10-26", title: "Kit release", added: "One thing\nAnother" }));
  await step(tl!, "PUT", `${ui}/changelog/${ch}`, { version: "9.9.9", released_on: "2026-10-26", title: "Kit release, edited", fixed: "A bug" });
  await step(tl!, "DELETE", `${ui}/changelog/${ch}`);
  const k = idOf(await step(tl!, "POST", `${ui}/keys`, { name: "kit ref", scope: "ref_ids" }));
  await step(tl!, "POST", `${ui}/keys/${k}/revoke`, {});
  const sec = idOf(await step(tl!, "POST", `${ui}/secrets`, { name: "KIT_TOKEN", value: "s3cret" }));
  await step(tl!, "PUT", `${ui}/secrets/${sec}`, { value: "s3cret-2" });
  await step(tl!, "DELETE", `${ui}/secrets/${sec}`);
  const card = idOf(await step(tl!, "POST", `${ui}/alert-cards`, { kind: "email", name: "Kit on call", critical: true, warning: true, emails: "kit@x.co" }));
  await step(tl!, "PUT", `${ui}/alert-cards/${card}/enabled`, { enabled: false });
  await step(tl!, "PUT", `${ui}/alert-cards/${card}`, { kind: "email", name: "Kit on call, edited", critical: true, warning: false, emails: "kit@x.co\nkit2@x.co" });
  await step(tl!, "DELETE", `${ui}/alert-cards/${card}`);

  // Tickets: team board, a reporter with a ref ID, stages.
  await step(tl!, "POST", `${ui}/tickets`, { title: "Kit bug", description: "Filed by the kit.", stage: "backlog" });
  const ref = (await step(write!, "POST", "/api/v1/ref-ids", { single_use: false })) as { data?: { ref_id?: string } };
  const refId = ref.data?.ref_id;
  if (refId) {
    const filed = (await step(p.anon!, "POST", "/api/v1/tickets", { ref_id: refId, title: "Kit report", description: "Card 4111 failed", page_url: "https://shop.example/pay" })) as { data?: { number?: number } };
    await step({ name: "reporter", headers: { "x-ref-id": refId } }, "GET", "/api/v1/tickets/mine");
    const num = filed.data?.number;
    if (num) {
      await step(tl!, "PUT", `${ui}/tickets/${num}/stage`, { stage: "in_progress" });
      await step(tl!, "PUT", `${ui}/tickets/${num}/public-title`, { public_title: "A card is declined" });
      await step(tl!, "PUT", `${ui}/tickets/${num}/stage`, { stage: "done", close_reason: "fixed" });
      await step(tl!, "POST", `${ui}/tickets/${num}/revoke-ref`, {});
    }
  }

  // People: invite, accept, role, disable, enable; avatar; two-factor on and off.
  const invite = (await step(admin!, "POST", "/api/ui/admin/users", { email: "kit-user@x.co" })) as { data?: { invite_url?: string; id?: number } };
  const inviteUrl = JSON.stringify(invite).match(/\/invite\/([A-Za-z0-9_-]+)/)?.[1];
  if (inviteUrl) {
    await step(p.anon!, "GET", `/api/ui/invites/${inviteUrl}`, undefined, "/api/ui/invites/{token}");
    await step(p.anon!, "POST", `/api/ui/invites/${inviteUrl}`, { password: "kit-password-1", password_confirm: "kit-password-1" }, "/api/ui/invites/{token}");
  }
  await step(admin!, "PUT", `/api/ui/admin/users/${ids.user}/role`, { role: "tl" });
  await step(admin!, "PUT", "/api/ui/me/avatar", { avatar: PNG });
  await step(admin!, "GET", "/api/ui/me");
  const tf = (await step(admin!, "POST", "/api/ui/me/two-factor", {})) as { data?: { secret?: string } };
  if (tf.data?.secret) {
    await step(admin!, "POST", "/api/ui/me/two-factor/confirm", { code: totp(tf.data.secret) });
    await step(admin!, "POST", "/api/ui/me/two-factor/off", { code: totp(tf.data.secret, Date.now() + 30_000) });
  }

  // Admin settings and a location with its probe.
  await step(admin!, "PUT", "/api/ui/admin/time-zone", { utc_offset_min: 420 });
  await step(admin!, "PUT", "/api/ui/admin/time-zone", { utc_offset_min: 0 });
  const loc = (await step(admin!, "POST", "/api/ui/admin/locations", { name: "Kit place" })) as { data?: { id?: number; token?: string } };
  if (loc.data?.token) {
    const probe: Who = { name: "probe", headers: { authorization: `Bearer ${loc.data.token}` } };
    await step(probe, "GET", "/api/v1/probe/work?free=10&wait=0");
    await step(probe, "POST", "/api/v1/probe/results", { results: [] });
    await step(admin!, "GET", `/api/ui/admin/locations/${loc.data.id}/monitors`);
    await step(admin!, "POST", `/api/ui/admin/locations/${loc.data.id}/token`, {});
    await step(admin!, "DELETE", `/api/ui/admin/locations/${loc.data.id}`);
  }

  // Deletes last: a monitor by key, a heartbeat, the kit's project.
  await step(write!, "DELETE", `${v1}/monitors/${tcp}`);
  await step(tl!, "DELETE", `${ui}/heartbeats/${hb}`);
  await step(admin!, "DELETE", `/api/ui/projects/${made.data?.slug ?? "missing"}`, { confirm: "Parity Kit" }, "/api/ui/projects/{slug}");
  await step(admin!, "POST", "/api/ui/logout", {});
}

/** Waits until the outbox has sent what the writes queued, so the reads after them do not race
 * the delivery loop. Checks of the kit's monitors hit a closed local port, so they fail at once
 * and the same way in every run. */
async function settle(admin: Who): Promise<void> {
  for (let i = 0; i < 100; i++) {
    const r = await call(admin, "GET", "/api/ui/admin/system");
    const email = (r.body as { data?: { email?: { waiting?: number } } }).data?.email;
    if (email && email.waiting === 0) break;
    await Bun.sleep(200);
  }
  await Bun.sleep(1500);
}

// ---------------------------------------------------------------- one run

async function run(label: string, bin: string, fixtureDir: string, ids: Ids): Promise<{ cap: Capture; dump: Record<string, Json[]> }> {
  const dir = join(OUT, label);
  rmSync(dir, { recursive: true, force: true });
  cpSync(fixtureDir, dir, { recursive: true });
  const db = join(dir, "monita.db");
  const proc = await start(bin, db);
  const cap: Capture = {};
  try {
    const index = await (await fetch(`${URL_}/`)).text();
    const assets = { js: index.match(/\/assets\/(main-[^"]+\.js)/)?.[1] ?? "missing.js" };
    const people: Record<string, Who> = {
      anon: { name: "anon", headers: {} },
      admin: { name: "admin", headers: { cookie: await login("admin@monita.test") } },
      tl: { name: "tl", headers: { cookie: await login("tl@monita.test") } },
      supervisor: { name: "supervisor", headers: { cookie: await login("supervisor@monita.test") } },
      read: { name: "read", headers: { authorization: `Bearer ${ids.read_key}` } },
      write: { name: "write", headers: { authorization: `Bearer ${ids.write_key}` } },
    };
    await settle(people.admin!);
    await reads(cap, "before", Object.values(people), ids, assets);
    await writes(cap, people, ids);
    await settle(people.admin!);
    await reads(cap, "after", [people.tl!, people.supervisor!, people.anon!], ids, assets);
  } finally {
    await stop(proc);
  }
  const integrity = await sql(db, "PRAGMA integrity_check;");
  const fk = await sql(db, "PRAGMA foreign_key_check;");
  const dump: Record<string, Json[]> = { _checks: [{ integrity, foreign_keys: fk }] };
  for (const t of (await sql(db, "SELECT name FROM sqlite_schema WHERE type = 'table' AND name NOT LIKE 'sqlite_%' ORDER BY name")).split("\n")) {
    const rows = await Bun.$`sqlite3 -json ${db} ${`SELECT * FROM "${t}" ORDER BY rowid`}`.text();
    dump[t] = rows.trim() ? JSON.parse(rows) : [];
  }
  dump._schema = (await sql(db, "SELECT type || ' ' || name FROM sqlite_schema WHERE name NOT LIKE 'sqlite_%' ORDER BY 1")).split("\n");
  writeFileSync(join(dir, "capture.json"), JSON.stringify(cap, null, 1));
  writeFileSync(join(dir, "dump.json"), JSON.stringify(dump, null, 1));
  return { cap, dump };
}

// ---------------------------------------------------------------- compare

type Finding = { where: string; base: Json; new: Json };

function walk(a: Json, b: Json, c: Json, at: string, noise: string[], bad: Finding[], ignore: (p: string) => boolean): void {
  if (ignore(at)) return;
  const same = (x: Json, y: Json): boolean => JSON.stringify(x) === JSON.stringify(y);
  if (same(a, c) || same(b, c)) return;
  const obj = (x: Json): x is Record<string, Json> => typeof x === "object" && x !== null;
  if (obj(a) && obj(b) && obj(c) && Array.isArray(a) === Array.isArray(c)) {
    const keys = new Set([...Object.keys(a), ...Object.keys(c)]);
    for (const k of keys) walk(a[k], b[k], c[k], `${at}.${k}`, noise, bad, ignore);
    return;
  }
  // A value that moves between two base runs (a clock, a token) is not compared, but its type
  // still is: an integer that comes back as text or null is a regression.
  const kind = (x: Json): string => (x === null ? "null" : Array.isArray(x) ? "array" : typeof x);
  if (!same(a, b) && kind(a) === kind(c)) noise.push(at);
  else if (!same(a, b) && kind(b) === kind(c)) noise.push(at);
  else bad.push({ where: at, base: a, new: c });
}

/** Differences the switch is meant to make. Each one is named here so nothing hides. */
const EXPECTED: ((p: string) => boolean)[] = [
  // How late the 1 s scheduler tick ran at the moment of the request: timing, not data.
  (p) => p.endsWith(".body.scheduler_lag_s"),
  (p) => /^dump\.__turso_internal_/.test(p),
  (p) => /^dump\._schema\b/.test(p),
  (p) => /^dump\.schema_migrations\.\d+$/.test(p),
];

function compare(b1: { cap: Capture; dump: Record<string, Json[]> }, nw: typeof b1, b2: typeof b1): { report: string; bad: number } {
  const noise: string[] = [];
  const bad: Finding[] = [];
  const ignore = (p: string): boolean => EXPECTED.some((f) => f(p));
  walk(b1.cap, b2.cap, nw.cap, "capture", noise, bad, ignore);
  walk(b1.dump, b2.dump, nw.dump, "dump", noise, bad, ignore);
  const reqs = Object.keys(nw.cap).filter((k) => !k.startsWith("SKIP"));
  const skipped = Object.keys(nw.cap).filter((k) => k.startsWith("SKIP"));
  const statuses = new Map<number, number>();
  for (const k of reqs) statuses.set(nw.cap[k]!.status, (statuses.get(nw.cap[k]!.status) ?? 0) + 1);
  const rows = Object.entries(nw.dump).filter(([k]) => !k.startsWith("_")).reduce((s, [, v]) => s + v.length, 0);
  const lines = [
    `# Parity report`,
    ``,
    `| | |`,
    `| --- | --- |`,
    `| Base binary | \`${BASE_BIN}\` |`,
    `| New binary | \`${NEW_BIN}\` |`,
    `| Requests compared | ${reqs.length} (status codes: ${[...statuses].sort().map(([s, n]) => `${s}×${n}`).join(", ")}) |`,
    `| Table rows compared | ${rows} in ${Object.keys(nw.dump).length - 2} tables |`,
    `| Integrity check, new run | ${JSON.stringify((nw.dump._checks as Json[])[0])} |`,
    `| Values that vary between two base runs (noise, masked) | ${noise.length} |`,
    `| **Regressions** | **${bad.length}** |`,
    ``,
  ];
  if (skipped.length) lines.push(`Routes not read (no value for a path parameter): ${skipped.map((s) => s.slice(5)).join(", ")}`, ``);
  if (bad.length) {
    lines.push(`## Regressions`, ``);
    for (const f of bad.slice(0, 200)) lines.push(`- \`${f.where}\`: base \`${JSON.stringify(f.base)?.slice(0, 200)}\`, new \`${JSON.stringify(f.new)?.slice(0, 200)}\``);
    if (bad.length > 200) lines.push(`- and ${bad.length - 200} more`);
  }
  lines.push(``, `## Noise (masked)`, ``, ...[...new Set(noise.map((n) => n.replace(/\.\d+(\.|$)/g, ".#$1")))].slice(0, 80).map((n) => `- \`${n}\``));
  return { report: lines.join("\n"), bad: bad.length };
}

// ---------------------------------------------------------------- main

// --compare-only reads the three runs already in --out: it tests the comparison itself, for
// example after planting a wrong value in new/capture.json (README.md).
const load = (label: string): { cap: Capture; dump: Record<string, Json[]> } => ({
  cap: JSON.parse(readFileSync(join(OUT, label, "capture.json"), "utf8")),
  dump: JSON.parse(readFileSync(join(OUT, label, "dump.json"), "utf8")),
});
let b1, nw, b2;
if (args["compare-only"]) {
  [b1, nw, b2] = [load("base-1"), load("new"), load("base-2")];
} else {
  mkdirSync(OUT, { recursive: true });
  const fixtureDir = join(OUT, "fixture");
  console.log(`fixture from ${BASE_BIN}`);
  const ids = await fixture(fixtureDir);
  console.log("run 1: base");
  b1 = await run("base-1", BASE_BIN, fixtureDir, ids);
  console.log("run 2: new");
  nw = await run("new", NEW_BIN, fixtureDir, ids);
  console.log("run 3: base");
  b2 = await run("base-2", BASE_BIN, fixtureDir, ids);
}
const { report, bad } = compare(b1, nw, b2);
writeFileSync(join(OUT, "report.md"), report);
console.log(report.split("\n## Noise")[0]);
console.log(`\nfull report: ${join(OUT, "report.md")}`);
if (existsSync(join(OUT, "new", "monita.db.log"))) {
  const errors = readFileSync(join(OUT, "new", "monita.db.log"), "utf8").split("\n").filter((l) => /ERROR|panicked/.test(l));
  if (errors.length) console.log(`\nnew binary logged ${errors.length} errors, first: ${errors[0]}`);
}
process.exit(bad ? 1 : 0);
