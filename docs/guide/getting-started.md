# Getting started

Run Monita on your laptop, add a project, and watch your first URL in about five minutes.

## What Monita does

Monita watches the systems of every project in a team and tells people when one breaks.

- An **HTTP monitor** requests a URL every 30 s to 5 min. Two failed checks in a row open an incident; the first passing check closes it.
- A **heartbeat** waits for a ping from a cron job. A ping that does not come in time opens an incident.
- An **incident** sends email and webhook alerts, and shows on the project's public **status page**.
- Users file **tickets** through the status page, and the tech lead sorts them on the ticket board.

A super admin invites tech leads. Each tech lead owns their projects and sees only those.

## Run it locally

You need Rust (the toolchain installs itself from `rust-toolchain.toml`).

```sh
git clone https://github.com/PT-Perkasa-Pilar-Utama/monita.git
cd monita
MONITA_RESET_TOKEN=dev make run
```

Monita serves on <http://127.0.0.1:8080> with its database in `./data/monita.db`. In a second terminal, load the sample accounts:

```sh
curl -s -X POST http://127.0.0.1:8080/admin/reset-state -H 'x-reset-token: dev' \
  -H 'content-type: application/json' -d '{"seed":"dev"}'
```

Sign in as `tl@monita.test` with the password `monita-dev-password`.

## Add a project

1. On **Projects**, enter a name and a slug. The slug is part of the status page URL: `/status/<slug>`.
2. Open the project. Its menu holds monitors, heartbeats, incidents, tickets, alerts and settings.

## Watch a URL

1. Open **Monitors** and select **New monitor**.
2. Enter a name and the URL. Keep the defaults: every minute, 10 s timeout, any `2xx` status.
3. Tick **Show on the public status page** if users may see it.
4. Save. The first check runs within one interval.

The monitor page shows the state, the response time chart and the availability. See [Monitors](monitors.md).

## Watch a cron job

1. Open **Heartbeats** and select **New heartbeat**.
2. Pick how often the job runs and how late it may be.
3. Save, then copy the ping URL into the job:

```sh
backup.sh && curl -fsS -m 10 https://monita.example.com/ping/<token>
```

See [Heartbeats](heartbeats.md) for failure reports and more examples.

## Next steps

- Set who gets alerts: [Incidents and alerts](incidents.md).
- Put Monita on a server: [Install](install.md).
