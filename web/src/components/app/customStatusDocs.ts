/** One public endpoint as the Custom status page section explains it. Contracts: api-specs 07
 * section 7.1 and 08 sections 8.1 to 8.3. */
export type EndpointDoc = {
  tab: string;
  method: "GET" | "POST" | "PUT";
  path: string;
  purpose: string;
  /** Who may call it and what they send to prove it, in plain words. */
  access: string;
  request: string;
  /** A response as the endpoint sends it, with a sample of each field; empty when it has no body. */
  response: string;
  responseTitle: string;
  /** Rows under the example, each a label and one or two plain sentences. */
  details: { label: string; text: string }[];
};

/** The error shape every endpoint shares. */
export const ERRORS = 'An error answers {"error": {"code": "...", "message": "..."}} with its HTTP status, and a 429 carries Retry-After in seconds.';

export const json = (v: unknown): string => JSON.stringify(v, null, 2);

export function endpointDocs(origin: string, slug: string, name: string): EndpointDoc[] {
  return [
    {
      tab: "Status",
      method: "GET",
      path: `/api/v1/public/status/${slug}`,
      purpose: "Reads the public systems, their daily uptime and the incidents, so your page can build everything from this one call.",
      access: "Anyone, with no sign-in or API key, and a browser may call it from any website.",
      request: `curl -s "${origin}/api/v1/public/status/${slug}?days=90"`,
      responseTitle: "Response 200",
      response: json({
        data: {
          project: { name, slug },
          overall: "down",
          items: [
            {
              id: 2,
              kind: "monitor",
              name: "API health",
              state: "down",
              uptime_90d: 99.95,
              days: [
                { day: "2026-10-18", uptime: 100.0, downtime_s: 0 },
                { day: "2026-10-19", uptime: 98.6, downtime_s: 1210 },
              ],
            },
          ],
          incidents: [
            {
              title: "API health is down",
              origin: "auto",
              severity: "critical",
              started_at: 1792396936,
              resolved_at: null,
              items: ["API health"],
              updates: [
                { label: "identified", message: "Database failover in progress.", at: 1792397236 },
                { label: "investigating", message: "Detected.", at: 1792396936 },
              ],
            },
          ],
          maintenance: [],
        },
      }),
      details: [
        { label: "Query", text: "days is optional, from 1 to 90 with a default of 90, and it sets the bars and how far back incidents go, while open incidents always come back." },
        { label: "States", text: "overall is up, down, issue or maintenance, or unknown while no public system was checked, and a system's state can also be unknown before its first check." },
        { label: "Bars", text: "days[] runs oldest first, and uptime is null for a day before the system existed." },
        { label: "Incidents", text: "updates[] runs newest first and label is investigating, identified, monitoring or resolved, while incidents is empty when the Incidents tab is off." },
        { label: "Maintenance", text: "The active windows that cover a public system, with title, message, starts_at, ends_at and items." },
        { label: "Shown", text: "Only systems marked public and not paused, with times in Unix seconds." },
        { label: "Limits", text: "60 calls a minute for each visitor address by default, and an answer can be up to 30 seconds old." },
        { label: "Errors", text: ERRORS },
      ],
    },
    {
      tab: "File a ticket",
      method: "POST",
      path: "/api/v1/tickets",
      purpose: "Files a bug report from your page or app, and it lands in Triage on the ticket board.",
      access: "Anyone who has a ref ID, which your page sends in the JSON body, and no API key is needed.",
      request: `curl -s -X POST "${origin}/api/v1/tickets" \\
  -H "Content-Type: application/json" \\
  -d '{"ref_id": "rf_5k2m9q7w3x8p1t6v4y0z2a8bcd", "title": "Export button does nothing", "description": "I click Export on the March report and nothing downloads.", "page_url": "https://app.example.com/reports/march"}'`,
      responseTitle: "Response 201",
      response: json({ data: { number: 17, stage: "triage", created_at: 1792137600 } }),
      details: [
        { label: "Body", text: "title takes 3 to 120 characters, description 1 to 10 000, and page_url is optional." },
        { label: "Limits", text: "5 tickets a day for one ref ID and 100 for the project by default, after which the answer is 429 daily_cap_reached." },
        { label: "Errors", text: `422 ref_id_invalid when the ref ID is unknown, revoked or used. ${ERRORS}` },
      ],
    },
    {
      tab: "Reporter's tickets",
      method: "GET",
      path: "/api/v1/tickets/mine",
      purpose: "Lists the tickets filed with one ref ID, newest first, so a reporter can follow them.",
      access: "Anyone who has the ref ID. Send it in the X-Ref-Id header, not in the URL, so it stays out of logs.",
      request: `curl -s "${origin}/api/v1/tickets/mine" \\
  -H "X-Ref-Id: rf_5k2m9q7w3x8p1t6v4y0z2a8bcd"`,
      responseTitle: "Response 200",
      response: json({
        data: [
          { number: 17, title: "Export button does nothing", stage: "in_progress", close_reason: null, created_at: 1792137600, updated_at: 1792224000 },
        ],
      }),
      details: [
        { label: "Fields", text: "stage is triage, backlog, in_progress or done, close_reason is fixed, wont_fix or duplicate once done, and the description never comes back." },
        { label: "Errors", text: `422 ref_id_invalid for an unknown ref ID. ${ERRORS}` },
      ],
    },
    {
      tab: "Issue a ref ID",
      method: "POST",
      path: "/api/v1/ref-ids",
      purpose: "Gives one of your users a ref ID, the code they file and follow tickets with.",
      access: "Your server only, with a Ref IDs only or a Read and write key sent as a Bearer token, and never put the key in a browser page.",
      request: `curl -s -X POST "${origin}/api/v1/ref-ids" \\
  -H "Authorization: Bearer $MONITA_API_KEY" \\
  -H "Content-Type: application/json" \\
  -d '{"single_use": false}'`,
      responseTitle: "Response 201",
      response: json({ data: { ref_id: "rf_5k2m9q7w3x8p1t6v4y0z2a8bcd", single_use: false, created_at: 1792137600 } }),
      details: [
        { label: "Body", text: "single_use is optional and defaults to false, and a single-use ref ID files one ticket." },
        { label: "Answer", text: "Store the ref ID with your user, because Monita shows it only in this answer." },
        { label: "Errors", text: `401 unauthorized when the key is missing, unknown or revoked. ${ERRORS}` },
      ],
    },
  ];
}
