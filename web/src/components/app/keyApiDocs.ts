import { ERRORS, json, type EndpointDoc } from "@/components/app/customStatusDocs";

/** The calls a Read or a Read and write key makes, as How API keys work shows them. The answers
 * are trimmed copies of real ones. Contract: api-specs 03 section 3.4. */
export function keyApiDocs(origin: string, slug: string): EndpointDoc[] {
  // The project's address in a variable keeps each request line inside the sheet.
  const base = `PROJECT="${origin}/api/v1/projects/${slug}"\n`;
  const auth = `  -H "Authorization: Bearer $MONITA_API_KEY"`;
  const body = (b: string): string => `${auth} \\\n  -H "Content-Type: application/json" \\\n  -d '${b}'`;
  return [
    {
      tab: "Monitors",
      method: "GET",
      path: `/api/v1/projects/${slug}/monitors`,
      purpose: "Reads the project's monitors with each one's state and last check, for example to wait until a deploy is healthy.",
      access: "A Read or a Read and write key.",
      request: `${base}curl -s "$PROJECT/monitors" \\\n${auth}`,
      responseTitle: "Response 200, trimmed",
      response: json({
        data: {
          monitors: [
            {
              id: 17,
              name: "Checkout API",
              state: "down",
              paused_at: null,
              last: { checked_at: 1792988303, ok: false, summary: "503 in 308ms" },
            },
          ],
          summary: { kind: "down", text: "1 of 50 monitors is down." },
        },
      }),
      details: [
        { label: "Fields", text: "state is up, down, issue, maintenance, paused or unknown before the first check, and each monitor also carries every field of its form." },
        { label: "Hidden", text: "A Read key gets header values and the request body as (hidden)." },
        { label: "Also", text: "GET /monitors/{id} reads one monitor in full, and /heartbeats and /heartbeats/{id} read heartbeats the same way." },
      ],
    },
    {
      tab: "Pause",
      method: "PUT",
      path: `/api/v1/projects/${slug}/monitors/{id}/paused`,
      purpose: "Stops a monitor or a heartbeat while you deploy or move it, so its failures open no incident and send no alert.",
      access: "A Read and write key.",
      request: `${base}curl -s -X PUT "$PROJECT/monitors/17/paused" \\\n${body('{"paused": true}')}`,
      responseTitle: "Response 204, no body",
      response: "",
      details: [
        { label: "Resume", text: "Send false to resume, and the monitor checks again at once." },
        { label: "While paused", text: "Nothing checks it, an open automatic incident closes without an alert, and it leaves the public status page until it resumes." },
        { label: "Heartbeats", text: "PUT /heartbeats/{id}/paused pauses a heartbeat with the same body, and a paused heartbeat still keeps the pings it gets." },
      ],
    },
    {
      tab: "Heartbeats",
      method: "POST",
      path: `/api/v1/projects/${slug}/heartbeats`,
      purpose: "Creates a heartbeat for a scheduled job, for example when a pipeline sets up a new one.",
      access: "A Read and write key.",
      request: `${base}curl -s -X POST "$PROJECT/heartbeats" \\\n${body('{"name": "nightly-backup", "period_s": "86400", "grace_s": "1800", "severity": "warning"}')}`,
      responseTitle: "Response 201",
      response: json({ data: { id: 55 } }),
      details: [
        { label: "Body", text: "period_s and grace_s are seconds, and severity is critical or warning." },
        { label: "Ping URL", text: "GET /heartbeats/{id} returns ping_url for the job to call, which only a Read and write key sees." },
        { label: "Also", text: "POST /monitors creates a monitor from the fields of the monitor form, and PUT or DELETE on /monitors/{id} and /heartbeats/{id} change or remove one." },
      ],
    },
    {
      tab: "Updates",
      method: "POST",
      path: `/api/v1/projects/${slug}/incidents/{id}/updates`,
      purpose: "Adds a status update to an incident, for example when your pipeline starts a rollback.",
      access: "A Read and write key.",
      request: `${base}curl -s -X POST "$PROJECT/incidents/10/updates" \\\n${body('{"label": "monitoring", "message": "The rollback is out, and we are watching the error rate."}')}`,
      responseTitle: "Response 204, no body",
      response: "",
      details: [
        { label: "Label", text: "investigating, identified, monitoring or resolved, and resolved closes the incident." },
        { label: "Author", text: "The team sees the key's name as the author, while the public page shows no author." },
        { label: "Also", text: "GET /incidents lists the open incidents with their ids, and POST /incidents opens one by hand." },
      ],
    },
    {
      tab: "Maintenance",
      method: "POST",
      path: `/api/v1/projects/${slug}/maintenance`,
      purpose: "Plans a maintenance window, so failures inside it open no incident and the status page shows Maintenance.",
      access: "A Read and write key.",
      request: `${base}curl -s -X POST "$PROJECT/maintenance" \\\n${body('{"title": "Database upgrade", "message": "Checkout may be slow.", "starts_at": 1793059200, "ends_at": 1793062800, "items": ["monitor:17"]}')}`,
      responseTitle: "Response 201",
      response: json({ data: { id: 4 } }),
      details: [
        { label: "Body", text: "starts_at and ends_at are Unix seconds, items name monitor:{id} or heartbeat:{id}, and message is optional." },
        { label: "Also", text: "POST /maintenance/{id}/end ends an active window, and DELETE /maintenance/{id} removes one that has not started." },
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
