import { ERRORS, json } from "@/components/app/customStatusDocs";

/** One route of a primitive, its path after `/api/v1/projects/{slug}`. */
export type KeyRoute = { method: "GET" | "POST" | "PUT" | "DELETE"; path: string; text: string; write: boolean };

export type KeyExample = { title: string; request: string; responseTitle: string; response: string };

/** One primitive as How API keys work shows it: every route a key may call, then worked examples. */
export type PrimitiveDoc = {
  tab: string;
  purpose: string;
  routes: KeyRoute[];
  examples: KeyExample[];
  details: { label: string; text: string }[];
};

/** The primitives a Read or a Read and write key manages. The answers are trimmed copies of real
 * ones. Contract: api-specs 03 section 3.4. */
export function keyApiDocs(origin: string, slug: string): PrimitiveDoc[] {
  // The project's address in a variable keeps each request line inside the sheet.
  const base = `PROJECT="${origin}/api/v1/projects/${slug}"\n`;
  const auth = `  -H "Authorization: Bearer $MONITA_API_KEY"`;
  const get = (path: string): string => `${base}curl -s "$PROJECT${path}" \\\n${auth}`;
  const send = (method: string, path: string, body: string): string =>
    `${base}curl -s -X ${method} "$PROJECT${path}" \\\n${auth} \\\n  -H "Content-Type: application/json" \\\n  -d '${body}'`;
  const none = "";
  return [
    {
      tab: "Monitors",
      purpose: "Checks of a URL, a TCP port, a DNS name or a ping, which a script can read before a deploy, pause during it and resume after.",
      routes: [
        { method: "GET", path: "/monitors", text: "Lists the monitors with each one's state and last check", write: false },
        { method: "GET", path: "/monitors/{id}", text: "Reads one monitor with its recent checks and stats", write: false },
        { method: "POST", path: "/monitors", text: "Creates a monitor from the fields of the monitor form", write: true },
        { method: "PUT", path: "/monitors/{id}", text: "Changes a monitor with the same fields", write: true },
        { method: "DELETE", path: "/monitors/{id}", text: "Deletes a monitor", write: true },
        { method: "PUT", path: "/monitors/{id}/paused", text: "Pauses it with {\"paused\": true} and resumes it with false", write: true },
      ],
      examples: [
        {
          title: "List monitors",
          request: get("/monitors"),
          responseTitle: "Response 200, trimmed",
          response: json({
            data: {
              monitors: [{ id: 17, name: "Checkout API", state: "down", paused_at: null, last: { checked_at: 1792988303, ok: false, summary: "503 in 308ms" } }],
              summary: { kind: "down", text: "1 of 50 monitors is down." },
            },
          }),
        },
        { title: "Pause a monitor while you deploy", request: send("PUT", "/monitors/17/paused", '{"paused": true}'), responseTitle: "Response 204, no body", response: none },
      ],
      details: [
        { label: "State", text: "up, down, issue, maintenance, paused, or unknown before the first check." },
        { label: "Paused", text: "Nothing checks it, an open automatic incident closes without an alert, and it leaves the public status page until it resumes and checks again at once." },
        { label: "Hidden", text: "A Read key gets header values and the request body as (hidden)." },
      ],
    },
    {
      tab: "Heartbeats",
      purpose: "Deadlines for scheduled jobs, which a pipeline can create for a new job and pause while the job is off.",
      routes: [
        { method: "GET", path: "/heartbeats", text: "Lists the heartbeats with each one's state and last ping", write: false },
        { method: "GET", path: "/heartbeats/{id}", text: "Reads one heartbeat with its pings, stats and ping URL", write: false },
        { method: "POST", path: "/heartbeats", text: "Creates a heartbeat from the fields of the heartbeat form", write: true },
        { method: "PUT", path: "/heartbeats/{id}", text: "Changes a heartbeat with the same fields", write: true },
        { method: "DELETE", path: "/heartbeats/{id}", text: "Deletes a heartbeat", write: true },
        { method: "PUT", path: "/heartbeats/{id}/paused", text: "Pauses it with {\"paused\": true} and resumes it with false", write: true },
      ],
      examples: [
        {
          title: "Create a heartbeat",
          request: send("POST", "/heartbeats", '{"name": "nightly-backup", "period_s": "86400", "grace_s": "1800", "severity": "warning"}'),
          responseTitle: "Response 201",
          response: json({ data: { id: 55 } }),
        },
        {
          title: "Read its ping URL",
          request: get("/heartbeats/55"),
          responseTitle: "Response 200, trimmed",
          response: json({
            data: {
              heartbeat: { id: 55, name: "nightly-backup", state: "unknown", period_s: 86400, grace_s: 1800, last_ping_at: null, paused_at: null },
              ping_url: `${origin}/ping/<token>`,
            },
          }),
        },
      ],
      details: [
        { label: "Body", text: "period_s and grace_s are seconds, and severity is critical or warning." },
        { label: "Ping URL", text: "Only a Read and write key sees ping_url, so a Read key gets null." },
        { label: "Paused", text: "No deadline runs and nothing opens, the pings it gets are still kept, and resuming starts a fresh deadline." },
      ],
    },
    {
      tab: "Incidents",
      purpose: "What went wrong and what the team says about it, which a pipeline can open and update while it rolls back.",
      routes: [
        { method: "GET", path: "/incidents", text: "Lists the open incidents, or ?state=resolved or ?state=all", write: false },
        { method: "GET", path: "/incidents/{id}", text: "Reads one incident with its items and updates", write: false },
        { method: "GET", path: "/incident-items", text: "Lists the monitors and heartbeats an incident may name", write: false },
        { method: "POST", path: "/incidents", text: "Opens an incident by hand with title, items, severity and message", write: true },
        { method: "POST", path: "/incidents/{id}/updates", text: "Posts an update with label and message", write: true },
      ],
      examples: [
        {
          title: "List open incidents",
          request: get("/incidents"),
          responseTitle: "Response 200, trimmed",
          response: json({
            data: {
              incidents: [{ id: 10, title: "Checkout API is down", origin: "auto", severity: "critical", started_at: 1792986803, resolved_at: null, last_label: "investigating" }],
            },
          }),
        },
        {
          title: "Post an update",
          request: send("POST", "/incidents/10/updates", '{"label": "monitoring", "message": "The rollback is out, and we are watching the error rate."}'),
          responseTitle: "Response 204, no body",
          response: none,
        },
      ],
      details: [
        { label: "Label", text: "investigating, identified, monitoring or resolved, and resolved closes the incident." },
        { label: "Items", text: "Each item is monitor:{id} or heartbeat:{id}." },
        { label: "Author", text: "The team sees the key's name as the author, while the public page shows no author." },
      ],
    },
    {
      tab: "Maintenance",
      purpose: "Planned work during which failures open no incident and the status page shows Maintenance.",
      routes: [
        { method: "GET", path: "/maintenance", text: "Lists the windows, 20 a page with ?page=", write: false },
        { method: "POST", path: "/maintenance", text: "Plans a window with title, message, starts_at, ends_at and items", write: true },
        { method: "POST", path: "/maintenance/{id}/end", text: "Ends an active window now", write: true },
        { method: "DELETE", path: "/maintenance/{id}", text: "Deletes a window that has not started", write: true },
      ],
      examples: [
        {
          title: "Plan a window",
          request: send("POST", "/maintenance", '{"title": "Database upgrade", "message": "Checkout may be slow.", "starts_at": 1793059200, "ends_at": 1793062800, "items": ["monitor:17"]}'),
          responseTitle: "Response 201",
          response: json({ data: { id: 4 } }),
        },
      ],
      details: [
        { label: "Body", text: "starts_at and ends_at are Unix seconds, items name monitor:{id} or heartbeat:{id}, and message is optional." },
        { label: "During it", text: "A covered failure opens no incident and sends no alert, while a missed heartbeat waits until the window ends." },
      ],
    },
  ];
}

/** What any key call can answer instead of success. */
export const KEY_ERRORS: { label: string; text: string }[] = [
  { label: "401", text: "No key, an unknown key or a revoked key." },
  { label: "403", text: "A key whose scope does not allow the call, or a caller outside the project's allowlist." },
  { label: "404", text: "Another project's slug, an unknown id, or a path the key API does not have, such as keys or secrets." },
  { label: "422", text: "A refused form, with each bad field named in error.fields." },
  { label: "429", text: "Too many calls for the key in a minute, with Retry-After in seconds." },
  { label: "Shape", text: ERRORS },
];
