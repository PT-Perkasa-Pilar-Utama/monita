# Status page

Each project has a public status page at `/status/<slug>`. It shows the monitors and heartbeats marked public, their last 90 days, and the incidents that touched them. Users need no sign-in.

Open the page from the project header, or from **Settings**, **Status page**.

## Tabs

The page has up to five tabs. A super admin or the owner picks them in **Settings**, **Status page**, under **Public tabs**.

| Tab | Shows | Setting |
| --- | --- | --- |
| Uptime | The systems and their 90 days | Always on |
| Incidents | Current problems and past incidents | Optional, on by default |
| Issues | The public ticket board | Optional |
| Ticket | A form to report a bug, and the reporter's tickets | Optional |
| Changelog | Releases, one version at a time | Optional, on by default for a new project |

Tick a card and select **Save changes**. A tab that is off disappears from the page. Its address and its JSON answer 404.

The header shows the project name, a link to the project's **Homepage** and the theme button. The tabs sit in the middle under it. A project made before the homepage field shows no link until someone sets the homepage in **Settings**, **General**.

## Uptime

- A headline: all systems operational, which system is down, that some have issues, or that maintenance is in progress. Beside it: "N of M systems up". While an incident is open and the Incidents tab is on, a link reads "N active incidents".
- Each system is a card with its name, a state icon, 90 daily bars, and "90 days ago", the uptime and "Today". A line under the bars says **Normal**, **Down**, **Has an issue**, **Under maintenance** or **No data yet**.
- Each active maintenance window shows under the headline with its title, times, the systems it covers and its message. See [Maintenance windows](incidents.md#maintenance-windows).
- A green bar had no downtime. A yellow bar had under 1% downtime. A red bar had more. A grey bar is a day before the system existed.
- A paused system does not show, on the page or in its JSON, until a tech lead resumes it.
- Systems with a problem come first, then systems in maintenance. A project with many systems shows bars for the problems and the first 12. The rest fold into a list, "N more systems".
- The tab reads itself every 60 seconds while the browser tab is visible. It has no **Refresh** button.

### Past uptime

Select **View past uptime**. Pick a public system, and the page shows three months as day grids, Sunday first. Each month shows its uptime. Use the arrows to move three months back or forward. A system that is not public answers 404.

## Incidents

- **Happening now** lists every open incident.
- **Past incidents** show three months at a time, with arrows to move. A month shows its first 3 resolved incidents. Select **Show all N incidents** to open the rest. A month with more than one page links to **Older incidents** in the history.
- Each row has a state icon, the title, the newest update, its label, and the time span. Select a row to read every update.
- The **Incident history** page lists every incident, newest first, 25 per page under month headings.

Users never see who posted an update, nor the technical cause Monita recorded. A manual incident shows as an issue, not downtime.

## Issues, Ticket and Changelog

- **Issues** is the public ticket board. See [Tickets](tickets.md#the-public-tabs).
- **Ticket** is where users report a bug. See [Tickets](tickets.md#the-public-tabs).
- **Changelog** shows the releases. See [Changelog](changelog.md).

On a phone the columns stack.

## Custom status page

**Settings**, **Status page** also holds **Custom status page**. It explains how to build your own page from the same data.

- **Download template** saves one HTML file, `status-<slug>.html`. It shows your systems and incidents and reloads every minute. Open it, or host it anywhere.
- Four tabs document the public endpoints: **Status**, **File a ticket**, **Reporter's tickets** and **Issue a ref ID**. Each tab shows the method and path, a `curl` request and an example response. The notes sit in labelled rows, such as Query, States, Bars, Incidents, Maintenance, Shown, Limits and Errors. The limits are defaults: a super admin changes them in **Admin**, **Rate limits**.

Only **Issue a ref ID** needs an API key, and your server must send it. The others are public. Never put an API key in a browser page.

Every error answers `{"error": {"code": "...", "message": "..."}}` with its HTTP status. A 429 carries `Retry-After` in seconds.

## Status JSON

Build your own status widget from the same data.

```sh
curl -s https://monita.example.com/api/v1/public/status/payments?days=30
```

| Part | Value |
| --- | --- |
| Access | Public, CORS `*`, cached 30 s |
| `days` | 1 to 90, default 90 |

```json
{
  "data": {
    "project": { "name": "Payments", "slug": "payments" },
    "overall": "down",
    "items": [
      { "id": 2, "kind": "monitor", "name": "API health", "state": "down", "uptime_90d": 99.95,
        "days": [ { "day": "2026-10-15", "uptime": 100.0, "downtime_s": 0 } ] }
    ],
    "incidents": [
      { "title": "API health is down", "origin": "auto", "severity": "critical",
        "started_at": 1792137600, "resolved_at": null, "items": ["API health"],
        "updates": [ { "label": "investigating", "message": "Detected.", "at": 1792137600 } ] }
    ]
  }
}
```

| Field | Values |
| --- | --- |
| `state` | `up`, `down`, `issue` (an open manual incident), `maintenance`, `unknown` (no data yet) |
| `overall` | `down` if any item is down, else `issue` if any has an issue, else `maintenance` if any is in maintenance, else `up` if any item was checked, else `unknown` (no public item, or none checked yet) |
| `days[].uptime` | `null` for a day before the item existed |
| `incidents` | Empty while the Incidents tab is off |

## Time zone

The page shows times and starts each day at midnight in the offset a super admin sets. See [Time zone](admin.md#time-zone).
