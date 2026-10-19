# Tickets

Users of a project report bugs to Monita. The tech lead sorts them on the ticket board, and the accepted ones show on a public board.

## How it fits together

1. A ref ID ties tickets to one user. Your backend asks Monita for one with the project's API key and gives it to the user, for example on their account page. A tech lead can also [issue one by hand](#issue-a-ref-id).
2. The user sends a ticket with that ref ID: from your app through the API, or through the **Ticket** tab at `/status/<slug>/ticket`, where they also check their tickets.
3. The ticket lands in **Triage**, which is private. The tech lead moves it to Backlog, In progress or Done.
4. Tickets out of Triage show on the public **Issues** tab at `/status/<slug>/issues`, under a public title if the tech lead set one.

The ref ID ties tickets to one user without Monita storing who they are. Monita keeps only its hash and its last 4 characters.

## Connect your app

On the **Tickets** page, select **Connect your app**. A panel shows six steps with the `curl` calls written for this project:

1. Make an API key.
2. Your server issues a ref ID for each user.
3. The user files a ticket with it.
4. Show the user their tickets.
5. Or link to the form Monita hosts.
6. Choose how to hear about new tickets, in [Ticket alerts](alerts.md#ticket-alerts).

The same calls follow here.

## Get an API key

In the project, open **Settings**, then **API keys**, and select **New API key**. Name the key after the program that uses it. Monita shows the key once. Every JSON call that needs a key sends it as a bearer token:

```sh
-H "Authorization: Bearer mk_..."
```

An IP allowlist in **Settings**, **Allowlist**, can limit where keys work. See [Project settings](project-settings.md).

## Issue a ref ID

Your server asks for a ref ID like this:

```sh
curl -s -X POST https://monita.example.com/api/v1/ref-ids \
  -H "Authorization: Bearer $MONITA_KEY" -H 'content-type: application/json' \
  -d '{"single_use": false}'
```

```json
{ "data": { "ref_id": "rf_5k2m9q7w3x8p1t6v4y0z2a8bcd", "single_use": false, "created_at": 1792137600 } }
```

A single-use ref ID takes one ticket. A reusable one takes up to 5 tickets a day.

### From the board

For someone who reports a bug outside your app, such as by phone or chat, select **Issue a ref ID** on the **Tickets** page.

1. Tick **It works for one report only** if you want a single-use ID.
2. Select **Issue ref ID**.
3. Copy the ref ID. Monita shows it only once.

The dialog says where the person uses it: the **Ticket** tab of your status page. If that tab is off, they can use it only through your app.

Only the project owner and a super admin can issue one.

## Submit a ticket

```sh
curl -s -X POST https://monita.example.com/api/v1/tickets -H 'content-type: application/json' \
  -d '{"ref_id": "rf_...", "title": "Checkout button does nothing",
       "description": "I click Pay and nothing happens.", "page_url": "https://shop.example.com/cart"}'
```

| Field | Rule |
| --- | --- |
| `ref_id` | Required |
| `title` | 3 to 120 characters |
| `description` | 1 to 10 000 characters. Never shown in public. |
| `page_url` | Optional, `http` or `https` |

The call needs no API key, so a browser can send it. CORS allows any origin. Answers: 201 with the ticket number, 422 `ref_id_invalid`, 429 `daily_cap_reached` (5 per ref ID, 100 per project per day).

## List a user's tickets

```sh
curl -s https://monita.example.com/api/v1/tickets/mine -H "X-Ref-Id: rf_..."
```

The user gets their newest 100 tickets with stage and close reason, never the description.

## The ticket board

The **Tickets** page shows four columns.

| Column | Public | Note |
| --- | --- | --- |
| Triage | No | Every new report starts here. Only your team sees it. |
| Backlog | Yes | Accepted, not started |
| In progress | Yes | Being fixed |
| Done | Yes | Needs a close reason: Fixed, Won't fix or Duplicate |

- Each column shows its newest 25 tickets and counts them all. Select **Show more** for the next 25.
- **See all N done tickets** opens a paged table of every Done ticket, with its close reason, report date and close date.
- Drag a card to another column. A drop on Done asks for a close reason first.
- Select a card to open the ticket in a side panel.
- A card shows its close reason and a **Public title** badge when the team set one.

### The ticket panel

| Part | Does |
| --- | --- |
| Header | The title, number, column, close reason and who filed it |
| Ref ID | The last 4 characters of the reporter's ref ID, **Active** or **Revoked**, and a **Revoke** button. Revoking stops new tickets from that ID. Old tickets stay. |
| Public title | Select the pencil to set a title the public board shows in place of the reporter's. Empty shows the reporter's title. |
| Report | The reporter's description and page URL. Only your team sees it. |
| Column | A select that saves at once. Moving to Done shows **Close reason**. |
| On the public board | How the card looks to visitors. A Triage ticket reads "Hidden until it leaves Triage." |
| Dates | When it was reported and last updated |

### File a ticket yourself

Select **New ticket** to file a ticket for your own team.

1. Enter a **Title** and a **Description**.
2. Pick a **Column**: Triage, Backlog or In progress. The default is Backlog.
3. Select **File ticket**.

A team-filed ticket has no reporter and no ref ID. It skips the daily caps. It sends no new-ticket webhook. The panel says "Filed by" with your email.

## The public tabs

Two public tabs belong to tickets. The owner turns each on under **Settings**, **Status page**.

- **Issues** shows Backlog, In progress and Done. A card shows the public title (or the reporter's title) and the number. It never shows the description or the reporter.
- **Ticket** has two cards. **Report a bug** takes **Your ref ID**, **What went wrong**, **What happened** and an optional **Page where it happened**. Select **Send report**. **Check your tickets** takes the ref ID and shows **Your tickets** with the column and close reason of each.

After a report, the page shows the reporter's tickets at once. The old addresses `/status/<slug>/report` and `/status/<slug>/mine` redirect to the Ticket tab.

## Across projects

**Tickets** in the top menu lists the new tickets in Triage across every project you can open. A tech lead sees their own projects. A super admin and a supervisor see all.

- The title reads "N in Triage".
- Search by title. Sort by **Reported**.
- The list shows 25 a page.
- A row opens the ticket in the same side panel, so closing it returns you to the same rows.

## Alerts

Ticket alerts have their own settings: the owner's daily digest, a webhook for each new ticket, and a webhook for each move. See [Alerts](alerts.md#ticket-alerts).
