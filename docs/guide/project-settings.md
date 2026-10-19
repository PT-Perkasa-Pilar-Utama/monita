# Projects and settings

A project groups the monitors, heartbeats, incidents and tickets of one system. It gets its own status page.

## The project list

**Projects** is the first page after sign-in. It lists your projects 20 at a time. A super admin and a supervisor see every project. A tech lead sees the projects they own.

- Switch between the card view and the list view. Monita remembers your choice. Both show the project's state, open incidents and open tickets. The card also shows the owner, the creation date and a link to the status page.
- **Search projects** runs on the server.
- Filter by **Owner** (only when more than one owner exists). Sort by name, **Newest first** or **Oldest first**.
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
