# Product

<!-- impeccable:product-schema 1 -->

## Platform

web

## Users

The primary user is any team that wants to host its own uptime monitoring. A team finds Monita as an open-source project and decides whether to run it on a server it already has. Inside that team, the tech lead of each project does the daily work: adding monitors and heartbeats, reading incidents, posting status updates, and sorting bug reports.

Monita started in the IT department of PT. Perkasa Pilar Utama, which is one such team.

Two other audiences use parts of the product:

- A super admin installs Monita, invites the tech leads, and can manage every project.
- The users of each project read its public status page and report bugs through it. They never sign in.

## Product Purpose

Monita tells a team that a service is down before its users write to them. It checks URLs and cron jobs, opens an incident after two failed checks in a row or a missed heartbeat, alerts the people the team lists by email and webhook, and shows the outage on a public status page. Users can report bugs from that page, and the tech lead sorts the reports on a ticket board.

Success means a team replaces a hosted service such as Better Stack with one program it runs itself, and connects each project to it in minutes.

## Positioning

Monita is a small, self-hosted Better Stack. It is one binary and one SQLite file, with no Docker, separate database or message queue. A project connects over plain HTTP: Monita calls a health URL, a cron job calls a ping URL, and a backend makes two API calls for bug reports. Monitors, heartbeats, incidents, status pages and bug tickets live in one place.

The motto from the scope decision is: easy to integrate, simple features, lightweight, fast, reliable, small footprint.

## Operating Context

- Monita runs on a Linux server next to the services it checks, behind the team's HTTPS reverse proxy. `monita setup` and `monita service install` set it up as a systemd service.
- Tech leads use the dashboard during the working day and when an alert arrives by email or in Slack or Microsoft Teams.
- The public status page is read by a project's users when something looks broken, often on a phone.
- The project site on GitHub Pages holds the landing page, the documentation (`docs/guide/`) and a static demo with sample data.

## Capabilities and Constraints

- HTTP monitors at 30, 60, 180 or 300 seconds. A 2xx answer within 10 seconds passes, and a monitor can also require a word in the body.
- Heartbeats with a period and a grace time. A call to `/ping/<token>/fail` opens an incident at once.
- Automatic incidents open after two failed checks by default (each monitor sets 1 to 99) and close on the first pass. Manual incidents close only when a tech lead posts "Resolved". Update labels: investigating, identified, monitoring, resolved.
- Alerts go through an outbox: email to listed addresses and one webhook per severity. A slow mail server never delays a check.
- Tickets move through triage, backlog, in progress and done. Triage is private. Ticket descriptions never appear in public.
- Every list has a hard cap, and every background job works in batches.
- Roles: super admin, tech lead and supervisor. A tech lead sees only their own projects. A supervisor sees every project and changes nothing.
- Version 1 does not include: checks from more than one place, TCP, ping or DNS checks, text-message or phone alerts, on-call rotations, single sign-on, two-factor sign-in, or file uploads on tickets.
- Terms have one meaning each. `docs/GLOSSARY.md` is the reference: say "tech lead", not "TL", and "ticket board", not "kanban".

## Brand Commitments

- The name is Monita. The license is MIT, and the copyright holder is PT. Perkasa Pilar Utama.
- The accent colour is magenta and marks the brand only: the mark, the main button, focus, links and the chart line. It never marks a state. The user rejected the generic AI purple.
- Copy is written in whole, literal sentences. It avoids metaphors, fragment patterns such as "No X, no Y", "Nothing ..." and "One X, one Y", and marketing filler.
- The user rejects common "AI slop" interface habits: eyebrow labels above headings, dot separators, decorative dots, and subtitles under form fields.

## Evidence on Hand

- The static demo with three sample projects (`seeds/demo.sql`, published at `/demo/`).
- Screenshots in `docs/assets/screens/`: monitor, monitors, incident, heartbeat, status page (desktop and phone), status history, changelog and ticket board. `e2e/demo/snapshot.ts --screens` regenerates them.
- Moni, the mascot, and one scene per primitive in `docs/assets/brand/`, drawn from the app's components by `make brand`.
- Measured on an Apple M1 on 2026-10-19 with a debug build: 500 monitors at a 60 s interval used 49 MB of memory and 1 to 8 % of one core. `docs/guide/install.md` turns this into the server requirements.
- Measured on macOS arm64 on 2026-10-17: the release binary is 25,789,648 bytes, and memory use stayed between 20.5 and 23.3 MiB while 100 monitors ran every 30 seconds for 142 seconds. The v0.1.0 Linux release (x86-64, static) is 31,508,016 bytes, in a 12,919,831-byte archive, which is above the 30 MB target in NFR-07. Memory on Linux has not been measured.
- A teammate can supply one real quote later. No quote exists yet; future pages must not invent one.
- v0.1.0 was released on GitHub on 2026-10-17 with a Linux x86-64 archive. v1.0.0 is tagged on 2026-10-19; from v1.0.0 the archive is `monita_linux_amd64.tar.gz`, so `releases/latest/download/monita_linux_amd64.tar.gz` always fetches the newest.
- There are no customers, user counts, testimonials, benchmarks against other tools, or press. Future pages must not invent them.

## Product Principles

1. Small is the feature. Every addition earns its place against the footprint, and a new dependency needs a reason a few lines of code cannot meet.
2. Connecting a project takes plain HTTP and minutes, with no library or agent to install.
3. State the truth plainly, including what Monita does not do.
4. An alert is never lost, and a check never waits on an alert.
5. Users' data stays private: secrets are stored only as hashes, and nothing private reaches a public page.

## Accessibility & Inclusion

- Text meets WCAG AA contrast in both light and dark themes, and secondary text in dark mode stays readable.
- Every page offers a light and dark theme switch.
- All motion respects `prefers-reduced-motion` and shows a still frame instead.
- Public pages work at phone width without sideways scrolling.
