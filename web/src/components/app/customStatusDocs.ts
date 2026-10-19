/** One public endpoint as the Custom status page section explains it. Contracts: api-specs 07
 * section 7.1 and 08 sections 8.1 to 8.3. */
export type EndpointDoc = {
  tab: string;
  method: "GET" | "POST";
  path: string;
  purpose: string;
  /** Who may call it and what they send to prove it, in plain words. */
  access: string;
  request: string;
  /** A response as the endpoint sends it, with a sample of each field. */
  response: string;
  responseTitle: string;
  notes: string[];
};

const json = (v: unknown): string => JSON.stringify(v, null, 2);

export function endpointDocs(origin: string, slug: string, name: string): EndpointDoc[] {
  return [
    {
      tab: "Status",
      method: "GET",
      path: `/api/v1/public/status/${slug}`,
      purpose: "Reads the public systems, their daily uptime and the incidents. Your page builds everything from this one call.",
      access: "Anyone. It needs no sign-in and no API key, and a browser may call it from any website.",
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
        },
      }),
      notes: [
        "days is optional: 1 to 90, default 90. It sets how many days of bars come back and how far back incidents go. Open incidents always come back.",
        "overall is up, down or issue. state is up, down, issue or unknown (no data yet).",
        "days[] runs oldest first. uptime is null for a day before the system existed.",
        "Times are Unix seconds. updates[] runs newest first; label is investigating, identified, monitoring or resolved.",
        "Only systems marked public appear. While the Incidents tab is off, incidents is empty.",
        "Each visitor address may call 60 times a minute. An answer can be up to 30 seconds old.",
      ],
    },
    {
      tab: "File a ticket",
      method: "POST",
      path: "/api/v1/tickets",
      purpose: "Files a bug report from your page or app. It lands in Triage on the ticket board.",
      access: "Anyone who has a ref ID. Your page sends the ref ID in the JSON body; no API key.",
      request: `curl -s -X POST "${origin}/api/v1/tickets" \\
  -H "Content-Type: application/json" \\
  -d '{"ref_id": "rf_5k2m9q7w3x8p1t6v4y0z2a8bcd", "title": "Export button does nothing", "description": "I click Export on the March report and nothing downloads.", "page_url": "https://app.example.com/reports/march"}'`,
      responseTitle: "Response 201",
      response: json({ data: { number: 17, stage: "triage", created_at: 1792137600 } }),
      notes: [
        "title: 3 to 120 characters. description: 1 to 10 000. page_url is optional.",
        "422 ref_id_invalid: the ref ID is unknown, revoked or already used.",
        "429 daily_cap_reached: 5 tickets a day for one ref ID, or 100 for the project.",
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
      notes: [
        "stage is triage, backlog, in_progress or done. close_reason is fixed, wont_fix or duplicate once done.",
        "The description never comes back. 422 ref_id_invalid for an unknown ref ID.",
      ],
    },
    {
      tab: "Issue a ref ID",
      method: "POST",
      path: "/api/v1/ref-ids",
      purpose: "Gives one of your users a ref ID, the code they file and follow tickets with.",
      access: "Your server only, with an API key from Settings, API keys, sent as a Bearer token. Never put the key in a browser page.",
      request: `curl -s -X POST "${origin}/api/v1/ref-ids" \\
  -H "Authorization: Bearer $MONITA_API_KEY" \\
  -H "Content-Type: application/json" \\
  -d '{"single_use": false}'`,
      responseTitle: "Response 201",
      response: json({ data: { ref_id: "rf_5k2m9q7w3x8p1t6v4y0z2a8bcd", single_use: false, created_at: 1792137600 } }),
      notes: [
        "single_use is optional, default false. A single-use ref ID files one ticket.",
        "Store the ref ID with your user. Monita shows it only in this answer.",
        "401 unauthorized: the key is missing, unknown or revoked.",
      ],
    },
  ];
}
