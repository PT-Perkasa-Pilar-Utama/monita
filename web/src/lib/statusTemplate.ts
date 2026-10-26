/**
 * A one-file status page a team can host anywhere. It reads Monita's public status JSON, which
 * needs no key and allows any origin, and redraws every 60 seconds. Text from the API goes in
 * with `textContent` only, so a title can never become markup.
 */
export function statusTemplate(origin: string, slug: string): string {
  // JSON.stringify quotes and escapes both values for the script; `<` is escaped too, so a value
  // can never close the script tag.
  const lit = (v: string): string => JSON.stringify(v).replace(/</g, "\\u003c");
  return `<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>Status</title>
<style>
  :root { --bg: #f7f8f8; --card: #fff; --ink: #16171a; --muted: #565a62; --line: #e3e4e6; --up: #23784e; --down: #d24545; --issue: #8a6300; --none: #e3e4e6; }
  @media (prefers-color-scheme: dark) {
    :root { --bg: #08090a; --card: #0f1011; --ink: #f7f8f8; --muted: #a9aeb6; --line: #23252a; --up: #4cb782; --down: #eb5757; --issue: #f2c94c; --none: #23252a; }
  }
  * { box-sizing: border-box; }
  body { margin: 0; background: var(--bg); color: var(--ink); font: 15px/1.5 system-ui, -apple-system, "Segoe UI", sans-serif; }
  main { max-width: 860px; margin: 0 auto; padding: 40px 16px; }
  h1 { font-size: 28px; margin: 0 0 24px; letter-spacing: -0.02em; }
  h2 { font-size: 18px; margin: 32px 0 12px; }
  .banner { border: 1px solid var(--line); border-radius: 12px; padding: 18px 22px; font-size: 19px; font-weight: 600; background: var(--card); }
  .banner.up { border-color: var(--up); } .banner.down { border-color: var(--down); } .banner.issue { border-color: var(--issue); } .banner.maintenance { border-color: var(--muted); }
  .card { background: var(--card); border: 1px solid var(--line); border-radius: 12px; overflow: hidden; }
  .row { padding: 16px 22px; border-top: 1px solid var(--line); }
  .row:first-child { border-top: 0; }
  .head { display: flex; justify-content: space-between; gap: 12px; font-weight: 600; }
  .state.up { color: var(--up); } .state.down { color: var(--down); } .state.issue { color: var(--issue); } .state.maintenance { color: var(--muted); } .state.unknown { color: var(--muted); }
  .bars { display: flex; gap: 2px; height: 32px; margin: 10px 0 6px; }
  .bars span { flex: 1; min-width: 0; border-radius: 2px; background: var(--none); }
  .bars .up { background: var(--up); } .bars .partial { background: var(--issue); } .bars .down { background: var(--down); }
  .muted { color: var(--muted); font-size: 13px; }
  .message { margin: 6px 0 0; white-space: pre-wrap; }
  footer { margin-top: 32px; color: var(--muted); font-size: 13px; }
</style>
</head>
<body>
<main>
  <h1 id="name">Status</h1>
  <p id="banner" class="banner">Loading…</p>
  <h2>Systems</h2>
  <div id="systems" class="card"></div>
  <section id="now-wrap" hidden>
    <h2>Happening now</h2>
    <div id="now" class="card"></div>
  </section>
  <section id="past-wrap" hidden>
    <h2>Recent incidents</h2>
    <div id="past" class="card"></div>
  </section>
  <footer id="updated"></footer>
</main>
<script>
const MONITA = ${lit(origin)};
const SLUG = ${lit(slug)};
const WORD = { up: "Up", down: "Down", issue: "Issue", maintenance: "Maintenance", unknown: "No data yet" };
const HEADLINE = { up: "All systems operational", down: "Some systems are down", issue: "Some systems have issues", maintenance: "Maintenance in progress", unknown: "No data yet" };

function el(tag, className, text) {
  const n = document.createElement(tag);
  if (className) n.className = className;
  if (text !== undefined) n.textContent = text;
  return n;
}

function tone(day) {
  if (day.uptime === null) return "";
  if (day.downtime_s === 0) return "up";
  return day.uptime >= 99 ? "partial" : "down";
}

function when(ts) {
  return new Date(ts * 1000).toLocaleString([], { dateStyle: "medium", timeStyle: "short" });
}

function system(item) {
  const row = el("div", "row");
  const head = el("div", "head");
  head.append(el("span", "", item.name), el("span", "state " + item.state, WORD[item.state] || item.state));
  const bars = el("div", "bars");
  for (const d of item.days) {
    const bar = el("span", tone(d));
    bar.title = d.day + (d.uptime === null ? ": no data" : ": " + d.uptime.toFixed(2) + "%");
    bars.append(bar);
  }
  const uptime = item.uptime_90d === null ? "No data yet" : item.uptime_90d.toFixed(2) + "% uptime";
  row.append(head, bars, el("div", "muted", uptime));
  return row;
}

function incident(i) {
  const row = el("div", "row");
  row.append(el("div", "head", i.title));
  const latest = i.updates[0];
  if (latest) row.append(el("p", "message", latest.message));
  const span = i.resolved_at === null ? "Since " + when(i.started_at) : when(i.started_at) + " to " + when(i.resolved_at);
  row.append(el("div", "muted", "Affects " + i.items.join(", ") + ". " + span));
  return row;
}

async function load() {
  try {
    const res = await fetch(MONITA + "/api/v1/public/status/" + encodeURIComponent(SLUG));
    if (!res.ok) throw new Error("HTTP " + res.status);
    const { data } = await res.json();
    document.title = data.project.name + " status";
    document.getElementById("name").textContent = data.project.name + " status";
    const banner = document.getElementById("banner");
    banner.className = "banner " + data.overall;
    banner.textContent = HEADLINE[data.overall] || data.overall;
    document.getElementById("systems").replaceChildren(...data.items.map(system));
    const open = data.incidents.filter((i) => i.resolved_at === null);
    const past = data.incidents.filter((i) => i.resolved_at !== null).slice(0, 5);
    document.getElementById("now-wrap").hidden = open.length === 0;
    document.getElementById("now").replaceChildren(...open.map(incident));
    document.getElementById("past-wrap").hidden = past.length === 0;
    document.getElementById("past").replaceChildren(...past.map(incident));
    document.getElementById("updated").textContent = "Updated " + new Date().toLocaleTimeString();
  } catch (err) {
    document.getElementById("updated").textContent = "Could not reach the status service and will retry in a minute.";
  }
}

load();
setInterval(load, 60000);
</script>
</body>
</html>
`;
}
