# Tickets

Users of a project report bugs to Monita. The tech lead sorts them on the ticket board, and the accepted ones show on a public board.

## How it fits together

1. Your backend asks Monita for a **ref ID** with the project's API key, and gives it to the user, for example on their account page.
2. The user sends a ticket with that ref ID: from your app through the API, or through the form at `/status/<slug>/report`.
3. The ticket lands in **Triage**, which is private. The tech lead moves it to Backlog, In progress or Done.
4. Tickets out of Triage show on the public board at `/status/<slug>/tickets`, under a public title if the tech lead set one.

The ref ID ties tickets to one user without Monita storing who they are. Monita keeps only its hash.

## Get an API key

In the project, open **Settings**, then **API keys**, and select **New API key**. Monita shows the key once. Every JSON call sends it as a bearer token:

```sh
-H "Authorization: Bearer mk_..."
```

An IP allowlist in **Settings**, **Allowlist**, can limit where keys work.

## Issue a ref ID

```sh
curl -s -X POST https://monita.example.com/api/v1/ref-ids \
  -H "Authorization: Bearer $MONITA_KEY" -H 'content-type: application/json' \
  -d '{"single_use": false}'
```

```json
{ "data": { "ref_id": "rf_5k2m9q7w3x8p1t6v4y0z2a8bcd", "single_use": false, "created_at": 1792137600 } }
```

A single-use ref ID takes one ticket. A reusable one takes up to 5 tickets a day.

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

| Stage | Public | Note |
| --- | --- | --- |
| Triage | No | Every new ticket starts here |
| Backlog | Yes | Accepted, not started |
| In progress | Yes | Being fixed |
| Done | Yes | Needs a reason: fixed, won't fix or duplicate |

Each column shows its newest 200 tickets and counts them all. Revoking a ticket's ref ID stops new tickets from it; old tickets stay.
