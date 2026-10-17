# Monitors

An HTTP monitor requests a URL on a fixed interval and checks the answer.

## What a check tests

| Setting | Values | Default |
| --- | --- | --- |
| Check every | 30 s, 1 min, 3 min, 5 min | 1 min |
| Timeout | 1 to 30 s | 10 s |
| Expected status | Classes such as `2xx`, codes such as `200`, or both: `2xx,301` | `2xx` |
| Keyword | Text the body must contain; Monita reads up to 1 MB of the body | none |
| TLS | Verified. Tick **Accept self-signed certificates** for internal hosts. | verified |
| Severity | Critical: email and webhook. Warning: email only. | warning |

A check fails on a timeout, a connection error, a status outside the expected list, or a missing keyword. Monita follows redirects, unless the expected status names a `3xx`: then the redirect itself is the answer.

The first check runs at a random moment within the first interval, so monitors added together do not check at the same second.

## When an incident opens

- Two failed checks in a row open an automatic incident and send alerts.
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
