# Monitors

An HTTP monitor requests a URL on a fixed interval and checks the answer.

## Create one

1. In a project, open **Monitors** and select **New monitor**. The form opens beside the list.
2. Enter a **Name**.
3. Fill in the **Request**, **Authentication** and **Headers** sections as you need.
4. Set what a passing check is under **A check passes when**.
5. Set what a failure does under **Alerts and status page**.
6. Save. The first check runs at a random moment within the first interval. Monitors added together do not check in the same second.

Select **How monitors work** at the top of the page for a short guide with examples.

## Request

| Field | Values | Default |
| --- | --- | --- |
| Method | GET, HEAD, POST or PUT | GET |
| URL | An `http://` or `https://` address | none |
| Request body | Text to send. Only POST and PUT send it. Up to 64 KB. | none |
| Check every | 30 seconds, 1 minute, 3 minutes or 5 minutes | 1 minute |
| Timeout in seconds | 1 to 30 | 10 |

Monita follows redirects, unless the expected status names a `3xx`. Then the redirect itself is the answer.

## Authentication

Under **Sign in with**, pick how the check proves who it is. The secret always comes from [Secrets](project-settings.md#secrets). Monita sends the value with each check and never shows it again.

| Option | Monita sends |
| --- | --- |
| None | Nothing extra |
| Bearer token | `Authorization: Bearer <secret>` |
| User name and password | HTTP basic sign-in. You type the **User name**. The **Password** is a secret. |
| A key in a header | The secret in a header you name, for example `X-Api-Key` |

A project with no secret yet shows a link to **Settings, Secrets**. A secret from another project is refused. If someone deletes the secret later, the check fails with a fixed sentence.

## Headers

Add plain headers as a name and a value. Use **Add header**. The limits are:

- Up to 20 headers.
- Up to 4 KB for one value.
- No `Host` or `Content-Length`. Monita sets those.
- No header that repeats the one Authentication sends.

Put a token under Authentication, not here. A header value is stored as the owner typed it.

## A check passes when

| Setting | Values | Default |
| --- | --- | --- |
| Status code | Classes such as `2xx`, codes such as `200`, or both: `2xx,301` | `2xx` |
| Response body contains | Plain text, matched with the same capitals and spaces anywhere in the first 1 MB of the body. Monita does not read JSON keys: for `{"status": "ok"}` enter `"status": "ok"`. | none |
| The certificate may be self-signed | Tick it for internal hosts | off (the certificate must be valid) |

A check fails on a timeout, a connection error, a status outside the expected list, or a missing text.

## When it fails

| Setting | Means |
| --- | --- |
| Severity | Critical alerts by email and webhook. Warning alerts by email only. Default: warning. |
| It counts as down after | The number of failed checks in a row, from 1 to 99. Default: 2. |
| Send alerts to the people in Alerts settings | Off means Monita sends nothing for this monitor. |
| Open an incident | Off means no incident and nothing on the status page. |
| Show this monitor and its incidents on the public status page | On puts the monitor on the status page. |

With **Open an incident** on, the monitor opens an incident after the set number of failed checks. The first passing check closes it and sends a recovery alert. The cause says "on 1 check" or "after N checks".

With **Open an incident** off, a failure only sends a `[DOWN]` alert, and recovery sends an `[UP]` alert. The team's lists show **Down**. The status page shows nothing. See [Alerts](alerts.md).

A tech lead can also open an incident by hand. See [Incidents](incidents.md).

## States

| State | Means |
| --- | --- |
| Up | The last check passed |
| Down | The monitor is past its failed-check count |
| Issue | An open manual incident names it |
| No data yet | No check has run |

## The monitor page

Select **Refresh** to read the latest data. The page also reads itself every 30 seconds.

| Part | Shows |
| --- | --- |
| Stats | How long it has been up or down, the last check, the mean response time, incidents in 30 days |
| Chart | Mean response time per bucket, failed checks in red, down time in a red band. Hover to read a bucket: the time, the count, and "Up" or "Down for ...". Hour, day and week read the raw checks; month reads the daily summary. |
| Availability | Today, the last 7, 30 and 365 days, and all time: percent up, downtime, incidents, the longest and the average |
| Checks | The last 7 days, 25 per page, newest first |

Raw checks are kept for 7 days. Each day is then kept as one summary row.

### Details

Select **Details** beside **Edit** for a side panel.

| Part | Shows |
| --- | --- |
| Request | The URL, method, sign-in, header names, body size, timeout and interval |
| Try it yourself | A `curl` command that sends the same request and prints the status code and time. For a signed-in monitor, its first line sets the secret as a shell variable. You fill in the value. |
| A check passes when | The status code, the body text and the certificate rule |
| When it fails | The failed-check count, what happens then, and the status page rule |
| Settings | The next check and the creation time |

The panel shows header names and a character count for the body. It never shows header values or secret values. A supervisor sees `(hidden)` for them.

## Limits

A project holds 100 monitors. See [Limits](limits.md).
