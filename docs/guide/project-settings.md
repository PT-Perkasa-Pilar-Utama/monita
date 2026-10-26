# Projects and settings

A project groups the monitors, heartbeats, incidents and tickets of one system. It gets its own status page.

## The project list

**Projects** is the first page after sign-in. It lists your projects 20 at a time. A super admin and a supervisor see every project. A tech lead sees the projects they own.

- Switch between the chart view (the default), the card view and the list view. Monita remembers your choice in this browser. The chart view draws the last 24 hours of the projects on the page in one chart, with the list under it. Each project's line is the mean response time of its monitors. The 5 that need you first are in colour: down, then issue, then the most failed checks, then the slowest. The rest are faint lines. Red shading shows when a monitor or heartbeat was down. Hover for every project in a 15-minute slot, and select a line or a name to open the project.
- The card and the list show the project's state, open incidents and open tickets. The card also shows the owner, the creation date and a link to the status page.
- Each project draws a bar for each of the last 24 hours. An hour before any monitor or heartbeat existed, or while all of them were paused, has no data and shows grey, not green.
- **Search projects** runs on the server.
- Filter by **State**: **Down** (a monitor or heartbeat is down), **Issue** (one is in an open manual incident and none is down) or **Up** (neither). The filter follows the badge each project shows. Filter by **Owner** (only when more than one owner exists). Sort by **Newest first** (the default), **Oldest first** or **Name**.
- Monita reopens your last search and page when you return to the list.
- **Refresh** reads the list again. The list also reads itself every 30 seconds.

## Create a project

Select **New project**.

| Field | Rule |
| --- | --- |
| Name | Required. Monita makes the slug from the name plus 6 random characters, for example `payments-k3x9qa`. The status page lives at `/status/<slug>`. |
| Homepage | Required. An `http://` or `https://` address, up to 2 000 characters. The public status page links to it. |
| Owner | Super admin only. A tech lead owns the projects they create. |
| Description | Optional, up to 280 characters |
| Icon | Optional. The page shrinks the picture to a small square before it sends it. |

The form names every bad field at once.

## The project tabs

A project has these tabs: **Overview**, **Monitors**, **Heartbeats**, **Incidents**, **Tickets**, **Changelog** and **Settings**. A count on a tab shows what needs you: monitors and heartbeats down, open incidents, and tickets in Triage.

### Overview

**Overview** opens with a **Last 24 hours** chart of every monitor and heartbeat, in 15-minute slots. A line under it says how many are down.

- **Lines**: the response time of up to 5 monitors that need you first. Down comes first, then issue, maintenance, no data yet and up. Then come the most failed checks, then the slowest. The scale is logarithmic, so a 5 ms DNS check and a 900 ms page both read.
- **Faint lines**: every other monitor, behind the lines. Hover one to name it.
- **Red shading**: a monitor was down, from its automatic incidents.
- **Lanes**: up to 6 heartbeats under the chart, with a dot for each slot that heard a ping. A red dot is a failed ping, and red shading is time missed or failed. The order is down first, then the most trouble, then the closest to its deadline. A last lane, such as **44 more**, holds the rest and still shows their failures.

Hover the chart to read each line and lane in that slot. Select a line, a faint line, a lane name or a legend entry to open its page. Paused monitors and heartbeats are left out, and the legend counts them. The page reads again every 30 seconds, and **Refresh** reads it now. The About column shows the owner's picture, the owner and the number of tickets in Triage.

## Settings

**Settings** has a side list with six pages. Each page has an **On this page** list when it has several sections.

### General

| Section | Does |
| --- | --- |
| Project | Shows the **Name** and **Slug**, which stay as they are. Edit the **Homepage**, **Description** and **Icon**, then select **Save changes**. Tick **Remove the icon and show the first letter** to drop a picture. |
| Owner | Shows the owner. A super admin selects **Reassign** and picks a new tech lead. The monitors, keys and tickets move with the project. The old owner loses access. |
| Danger zone | Super admin only. **Delete this project** removes its monitors, heartbeats, incidents, tickets, keys and status page. Type the project name to confirm. This cannot be undone. |

A project made before the homepage field existed shows no homepage until someone sets one.

### Status page

Choose the public tabs and build a custom page. See [Status page](status-page.md).

### Alerts

Set who gets critical, warning and ticket alerts. See [Alerts](alerts.md).

### API keys

An API key lets your backend call the Monita API, for example to issue ref IDs. See [Tickets](tickets.md#get-an-api-key).

1. Select **New API key**.
2. Name it after the program that uses it, up to 60 characters.
3. Copy the key. Monita shows it only once.

Select **How API keys work** for a guide. It says what a key is for: your own server issues ref IDs. A key cannot read or change monitors, incidents or settings. It shows the three steps of the ticket flow, and how to send a key: `Authorization: Bearer mk_…`, from your server only. A key is shown once. A revoked key gets 401. A key has 120 calls a minute by default. Tabs show the calls to **Issue a ref ID**, **File a ticket** and **Reporter's tickets**.

The table lists each key's name, a short form of the key, **Last used** and **Status**. Select **Revoke** to stop a key. A revoked key gets 401 on every call. This cannot be undone. A project holds 20 live keys.

The key starts with `mk_`. Monita stores only its hash.

### Secrets

A secret is a token or password that a monitor sends with each check. See [Monitors](monitors.md#authentication).

| Action | Does |
| --- | --- |
| New secret | Takes a **Name** and a **Value**. The name has 1 to 60 letters, digits, `_` or `-`, for example `REPORT_API_TOKEN`. The value has up to 4 KB. |
| Replace value | Saves a new value under the same name. Monitors that use it keep working. |
| Delete | Removes the secret. Monita refuses while a monitor uses it. Change those monitors first. |

The table lists each name, **Used by** (the number of monitors), and **Updated**. Monita never shows a value again. A project holds 20 secrets.

Monita stores a value as entered, so a person with access to the database file can read it. The page lists names only. A supervisor can read the list but change nothing.

### Allowlist

By default, any address can use the project's API keys. To limit them, enter **Allowed addresses**: one IP address or CIDR range per line, for example `10.20.0.0/16`. An empty list allows any address. Save an empty list to turn the allowlist off.

## Roles in a project

| Role | Can |
| --- | --- |
| Super admin | Everything, in every project |
| Tech lead | Everything in the projects they own. A project they do not own answers 404. |
| Supervisor | Read every project. Change nothing. |

See [Admin and roles](admin.md).
