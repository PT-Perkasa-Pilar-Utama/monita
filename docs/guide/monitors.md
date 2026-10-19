# Monitors

An HTTP monitor requests a URL on a fixed interval and checks the answer.

## What a check tests

| Setting | Values | Default |
| --- | --- | --- |
| Check every | 30 seconds, 1 minute, 3 minutes, 5 minutes | 1 minute |
| Timeout | 1 to 30 seconds | 10 |
| Status code | Classes such as `2xx`, codes such as `200`, or both: `2xx,301` | `2xx` |
| Response body contains | Plain text, matched with the same capitals and spaces anywhere in the first 1 MB of the body. Monita does not read JSON keys: for `{"status": "ok"}` enter `"status": "ok"`. | none |
| TLS | Verified. Tick **The certificate may be self-signed** for internal hosts. | verified |
| Severity | Critical: email and webhook. Warning: email only. | warning |

A check fails on a timeout, a connection error, a status outside the expected list, or a missing keyword. Monita follows redirects, unless the expected status names a `3xx`: then the redirect itself is the answer.

The first check runs at a random moment within the first interval, so monitors added together do not check at the same second.

## When an incident opens

- 2 failed checks in a row open an automatic incident and send alerts. Enter another count, from 1 to 99, under **When it fails**.
- The first passing check closes it and sends a recovery alert.
- A tech lead can also open an incident by hand. See [Incidents and alerts](incidents.md).

## The monitor page

| Part | Shows |
| --- | --- |
| Stats | How long it has been up or down, the last check, the mean response time, incidents in 30 days |
| Chart | Mean response time per bucket, failed checks in red, down time in a red band. Hour, day and week read the raw checks; month reads the daily summary. |
| Availability | Today, the last 7, 30 and 365 days, and all time: percent up, downtime, incidents, the longest and the average |
| Checks | The last 7 days, 25 per page, newest first |

Raw checks are kept for 7 days. Each day is then kept as one summary row.

## Limits

A project holds 100 monitors. See [Limits](limits.md).
