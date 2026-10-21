# Heartbeats

A heartbeat watches a job that runs on a schedule. The job calls a URL when it finishes. If no call comes in time, Monita opens an incident.

## Create one

1. In a project, open **Heartbeats** and select **New heartbeat**.
2. Enter a **Name**, for example `nightly-backup`.
3. Set **Runs every** to the job's schedule: 1, 5, 10, 15 or 30 minutes, 1, 2, 6 or 12 hours, 1, 7 or 30 days.
4. Set **Can be late by** to how late the job may finish: None, 1, 5, 15 or 30 minutes, 1, 2 or 6 hours, or 1 day.
5. Pick the alert level and what a failure does. These fields work as on a [monitor](monitors.md#when-it-fails), but a heartbeat has no failed-check count.
6. Save, then copy the ping URL.

The deadline is the last ping plus **Runs every** plus **Can be late by**. Example: a job runs every 1 day, can be late by 1 hour, and pings at 01:00. The next deadline is 02:00 the next day.

The first deadline counts from the moment you save. A heartbeat that never pings still misses its first deadline.

## The lists

The **Heartbeats** page lists each heartbeat with its state, **Next ping due**, **Every**, **Alert level** and **Last ping**. A heartbeat with no ping yet reads **No data yet**.

## Ping on success

Call the URL with `GET`, `HEAD` or `POST` when the job succeeds. Monita ignores the body of a success ping. Each ping moves the deadline to now plus **Runs every** plus **Can be late by**, and closes an open incident.

```sh
curl -fsS -m 10 https://monita.example.com/ping/<token>
```

## Report a failure

Add `/fail` to report a failure at once. Monita opens an incident without waiting for the deadline. Send the reason as plain text in the body. Monita keeps the first 1 KB as the incident's cause.

```sh
if ! output=$(backup.sh 2>&1); then
  printf '%s' "$output" | curl -fsS -m 10 --data-binary @- https://monita.example.com/ping/<token>/fail
  exit 1
fi
curl -fsS -m 10 https://monita.example.com/ping/<token>
```

## The Details panel

On a heartbeat page, select **Details** beside **Edit**. A heartbeat with no ping yet shows a card with the URL and a **How to call it** button that opens the same panel.

| Part | Shows |
| --- | --- |
| Ping URL | The URL, with a copy button |
| Call it from your job | Three blocks to copy, in **Shell**, **Node.js** or **Python**. Monita remembers your choice in this browser. |
| Settings | **Runs every**, **Can be late by**, what a failure does, the status page rule and the creation time |

The three blocks are:

- **When the job succeeds**: one call to the URL.
- **When it fails, with the reason**: one call to the `/fail` URL with a body.
- **Run the job and report either**: a script that runs the job, pings on success, and sends the error as the reason on failure.

The Node.js and Python scripts look like this:

```js
// Node 18 or later. Runs the job, then tells Monita how it went.
const PING = "https://monita.example.com/ping/<token>";
try {
  await runJob();
  await fetch(PING, { signal: AbortSignal.timeout(10_000) });
} catch (err) {
  const reason = String(err?.stack ?? err).slice(-1000);
  await fetch(`${PING}/fail`, { method: "POST", body: reason, signal: AbortSignal.timeout(10_000) });
  throw err;
}
```

```python
import traceback
import urllib.request

PING = "https://monita.example.com/ping/<token>"

try:
    run_job()
    urllib.request.urlopen(PING, timeout=10)
except Exception:
    reason = traceback.format_exc()[-1000:].encode()
    urllib.request.urlopen(PING + "/fail", data=reason, timeout=10)
    raise
```

A supervisor does not see the ping URL.

## Pause a heartbeat

Select **Pause** on the heartbeat page and confirm. While it is paused:

- Monita still accepts its pings and stores them, but a ping opens and closes nothing.
- Monita watches no deadline, so a missed ping opens no incident and sends no alert.
- The state reads **Paused** in grey, and the page says **Paused since** and the time.
- The status page hides it.

Pausing closes the heartbeat's open automatic incident without a resolved alert. Its last update reads "Paused by a tech lead." Select **Resume** and the deadline starts over: now plus **Runs every** plus **Can be late by**.

For planned work that should not alert, you can also [plan maintenance](incidents.md#maintenance-windows) from the heartbeat page.

## Resolve by hand

A tech lead can post **Resolved** on a heartbeat's incident, for example when the job is retired. The heartbeat gets a new deadline: now plus **Runs every** plus **Can be late by**. A silent job opens a new incident at that time. See [Incidents](incidents.md#resolve-an-automatic-incident-by-hand).

## More examples

A crontab line:

```sh
0 2 * * * /opt/jobs/backup.sh && curl -fsS -m 10 https://monita.example.com/ping/<token>
```

A systemd service run by a timer:

```ini
[Service]
Type=oneshot
ExecStart=/opt/jobs/backup.sh
ExecStartPost=/usr/bin/curl -fsS -m 10 https://monita.example.com/ping/<token>
```

A Kubernetes CronJob container:

```yaml
command: ["/bin/sh", "-c", "run-report && wget -q -T 10 -O /dev/null https://monita.example.com/ping/<token>"]
```

## Rules

| Rule | Value |
| --- | --- |
| Pings per heartbeat | 1 per second by default. Extra pings get 429 and are not stored. The ping accepted in that second keeps the heartbeat alive. `/fail` has its own limit, so a fail right after a success is heard. A super admin changes this on [Rate limits](admin.md#rate-limits). |
| Unknown tokens | 60 per minute per IP, then 429 |
| Request body | Read up to 16 MiB within 10 s; only `/fail` keeps its first 1 KB |
| Pings kept | 7 days |

The token is the secret. Anyone with the URL can ping. Rotate it by deleting the heartbeat and making a new one.
