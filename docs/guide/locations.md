# Locations

Every check runs from the server that runs Monita. If that server's network fails, every monitor looks down. A location is another machine that runs checks for this Monita and reports the results. A monitor can check from this server and from any locations, and count a check as failed only when enough of those places agree.

## Main and probe

The main Monita is the one your team uses. A probe is the same binary, started as `monita probe` on another machine. It is not a second Monita and not a backup.

| | Main Monita | Probe |
| --- | --- | --- |
| Who uses it | Everyone: pages, projects, incidents, alerts, the status page | Nobody. It has no pages and no sign-in. |
| Data | The database | None. It keeps only its token. |
| Checks | Every monitor | Only the monitors that tick its location |
| Decides "down" | Yes. It weighs every place, then opens incidents and sends alerts. | No. It reports pass or fail. |
| Heartbeats | Receives the pings | Not involved |
| If it stops | Monita is down | Its monitors carry on from the other places, and its row reads **Not running** |

There is no sync between them. The probe asks the main Monita for work and gets each check in full: the URL, the method, the headers and any secret. It runs the check and sends the result back.

```text
probe ── "any work for Jakarta?" (held up to 25 s) ──► main
probe ◄── the checks that fall due ──────────────────── main
probe ── "monitor 7: failed, 503, 40 ms" ────────────► main
```

The probe calls out over HTTPS. The main Monita never calls the probe, so the probe can sit behind a firewall or NAT.

## Add a location

You must be a super admin.

1. Open **Admin**, then **Locations**.
2. Select **Add location** and enter a name, such as `Jakarta`.
3. A panel opens with the commands for that machine. Your Monita's address and the new token are already in them. Monita shows the token only in this panel.
4. Run one of the commands on the location's machine (see below).
5. Keep the panel open. It shows **Jakarta is connected** when the probe makes its first call, within a few seconds.

A Monita has 10 locations at most.

## Run the probe

The panel has three tabs. Each runs the same probe.

| Tab | Use it when |
| --- | --- |
| **Docker** | The machine has Docker. The container starts again after a restart. |
| **Linux service** | The machine runs Linux with systemd. The commands download Monita and run `monita probe install`, which starts the probe with the machine. |
| **Run once** | You want to try it in a terminal. It stops when the terminal closes. |

Docker:

```sh
docker run -d --name monita-probe --restart unless-stopped \
  -e MONITA_PROBE_SERVER=https://monita.company.co.id \
  -e MONITA_PROBE_TOKEN='ml_…' \
  ghcr.io/pt-perkasa-pilar-utama/monita:1 probe
```

Linux service:

```sh
curl -fsSL https://github.com/PT-Perkasa-Pilar-Utama/monita/releases/latest/download/monita_linux_$(uname -m | sed 's/x86_64/amd64/;s/aarch64/arm64/').tar.gz | tar -xz monita
sudo install -m 0755 monita /usr/local/bin/monita
sudo MONITA_PROBE_TOKEN='ml_…' monita probe install --server https://monita.company.co.id
```

`monita probe install` writes the token to `/etc/monita/probe.env` with mode 0600 and the unit to `/etc/systemd/system/monita-probe.service`, then starts it. Follow it with `journalctl -u monita-probe -f`. `sudo monita probe uninstall` stops and removes it. The unit is also in [deploy/monita-probe.service](../../deploy/monita-probe.service) for a setup by hand.

| Option | Default | Meaning |
| --- | --- | --- |
| `--server` | `MONITA_PROBE_SERVER` | The address of your Monita |
| `--token` | `MONITA_PROBE_TOKEN` | The token. Use the variable: a command line shows in `ps`. |
| `--concurrency` | 20 | Checks at once, 1 to 50 |

The probe asks for work with a long poll: the main Monita holds the request until a check falls due, for 25 seconds at most. An idle probe so calls about twice a minute. Results go back as each check finishes. If your Monita cannot be reached, the probe tries again 5 seconds later. If the location was deleted or got a new token, the probe stops with an error.

**Treat the token as a password.** A probe receives the secret value of every monitor that signs in and checks from its location.

## New token

Select **⋯**, then **New token**, on a location's row. The old token stops at once, and the same setup panel opens with the new one. Use it to move the probe to another machine, or when the token is lost. The location reads **Not running yet** until the probe calls with the new token.

## Check from a location

1. Edit a monitor. **Check from** lists this server and each location.
2. Tick the locations.
3. Set **A check counts as failed when** to the number of places that must fail.

| Places | Fail when | Result |
| --- | --- | --- |
| This server, Jakarta, Tokyo | 2 | One place failing is a blip. Two is an outage. |
| This server, Jakarta | 1 | Any failing place counts, as before. |
| This server, Jakarta | 2 | Both must fail. A local network fault alone opens no incident. |

Only fresh results count: a place's newest result counts for two intervals and 30 seconds. When a probe goes silent, the places that still answer decide, so a lost probe never hides an outage. Monita weighs the places at each check from this server, so **It counts as down after** still means checks in a row, one per interval. A failure only a location sees counts at this server's next check.

## What the pages show

- **Locations** shows each location's **State**: **Running**, or **Not running** with when it last called, after two minutes of silence. Monita does not alert on a silent probe. To be alerted, have the probe's machine ping a Monita heartbeat.
- **Checks** counts the monitors that check from the location. Select it to list them, each with its project and a link.
- The monitor page shows **Places**: each place's newest result, and whether it is too old to count. The check list names the place of each check.
- Charts, daily uptime and the response time use this server's checks only, so the numbers stay comparable.

## Load on the main Monita

An idle probe costs the main Monita about two requests a minute and a small read every 5 seconds. On a debug build on a laptop, 8 idle probes used 0.55 s of CPU a minute, under 1% of one core. With the old one-second polling, the same 8 used 3.04 s.

Deleting a location stops its probe, deletes its results, and lowers each monitor's count to the places left.
