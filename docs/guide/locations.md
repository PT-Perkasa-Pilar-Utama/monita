# Locations

Every check runs from the server that runs Monita. If that server's network fails, every monitor looks down. A location is another machine that runs checks for this Monita and reports the results. A monitor can check from this server and from any locations, and count a check as failed only when enough of those places agree.

## Add a location

You must be a super admin.

1. Open **Admin**, then **Locations**.
2. Select **Add location** and enter a name, such as `Jakarta`.
3. Copy the probe command. Monita shows the location's token only once.

A Monita has 10 locations at most.

## Run the probe

On the location's machine, download Monita as the [install guide](install.md) shows. The probe needs no database, no env file and no setup. Run:

```sh
export MONITA_PROBE_TOKEN='ml_…'
./monita probe --server https://monita.company.co.id
```

| Option | Default | Meaning |
| --- | --- | --- |
| `--server` | `MONITA_PROBE_SERVER` | The address of your Monita |
| `--token` | `MONITA_PROBE_TOKEN` | The token. Use the variable: a command line shows in `ps`. |
| `--concurrency` | 20 | Checks at once, 1 to 50 |

The probe asks for work each second and posts results as checks finish. It needs outbound HTTPS to your Monita only; the location can be behind NAT. If your Monita cannot be reached, the probe tries again 5 seconds later. If the location was deleted, the probe stops with an error.

To keep the probe running, use a systemd unit such as:

```ini
[Unit]
Description=Monita probe
After=network-online.target

[Service]
Environment=MONITA_PROBE_SERVER=https://monita.company.co.id
EnvironmentFile=/etc/monita/probe.env
ExecStart=/usr/local/bin/monita probe
Restart=always
DynamicUser=yes

[Install]
WantedBy=multi-user.target
```

Put `MONITA_PROBE_TOKEN=ml_…` in `/etc/monita/probe.env` with mode 0600.

**Treat the token as a password.** A probe receives the secret value of every monitor that signs in and checks from its location.

## Check from a location

1. Edit a monitor. **Check from** lists this server and each location.
2. Tick the locations.
3. Set **A check counts as failed when** to the number of places that must fail.

| Places | Fail when | Result |
| --- | --- | --- |
| This server, Jakarta, Tokyo | 2 | One place failing is a blip. Two is an outage. |
| This server, Jakarta | 1 | Any failing place counts, as before. |
| This server, Jakarta | 2 | Both must fail. A local network fault alone opens no incident. |

Only fresh results count: a place's newest result counts for two intervals and 30 seconds. When a probe goes silent, the places that still answer decide, so a lost probe never hides an outage. The failure count in a row (**It counts as down after**) counts these verdicts, one per result from any place.

## What the pages show

- **Locations** shows each location's **Last seen**. A location silent for two minutes reads in red. Monita does not alert on a silent probe. To be alerted, have the probe's machine ping a Monita heartbeat.
- The monitor page shows **Places**: each place's newest result, and whether it is too old to count. The check list names the place of each check.
- Charts, daily uptime and the response time use this server's checks only, so the numbers stay comparable.

Deleting a location stops its probe, deletes its results, and lowers each monitor's count to the places left.
