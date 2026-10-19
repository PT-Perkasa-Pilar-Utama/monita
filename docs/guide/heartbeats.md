# Heartbeats

A heartbeat watches a job that runs on a schedule. The job calls a URL when it finishes. If no call comes in time, Monita opens an incident.

## Create one

1. In a project, open **Heartbeats** and select **New heartbeat**.
2. Set **Runs every** to the job's schedule, from 1 minute to 30 days.
3. Set **Can be late by** to how late the job may finish, up to 1 day.
4. Save and copy the ping URL.

The deadline is the last ping plus **Runs every** plus **Can be late by**. Example: a job runs every 1 day, can be late by 1 hour, and pings at 01:00. The next deadline is 02:00 the next day. The first deadline counts from the moment you save. A heartbeat that never pings still misses its first deadline.

## Ping on success

Call the URL with `GET`, `HEAD` or `POST` when the job succeeds. Each ping moves the deadline to now plus **Runs every** plus **Can be late by**, and closes an open incident.

```sh
curl -fsS -m 10 https://monita.example.com/ping/<token>
```

## Report a failure

Add `/fail` to report a failure at once. Monita opens an incident without waiting for the deadline. The first 1 KB of the request body becomes the incident's cause.

```sh
if ! output=$(backup.sh 2>&1); then
  printf '%s' "$output" | curl -fsS -m 10 --data-binary @- https://monita.example.com/ping/<token>/fail
  exit 1
fi
curl -fsS -m 10 https://monita.example.com/ping/<token>
```

## Examples

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
| Pings per heartbeat | 1 per second. Extra pings get 429 and are not stored. `/fail` has its own limit, so a fail right after a success is heard. |
| Unknown tokens | 60 per minute per IP, then 429 |
| Request body | Read up to 16 MiB within 10 s; only `/fail` keeps its first 1 KB |
| Pings kept | 7 days |

The token is the secret. Anyone with the URL can ping. Rotate it by deleting the heartbeat and making a new one.
