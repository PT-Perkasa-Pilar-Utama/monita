# Monitors

A monitor checks one thing on a fixed interval: a URL over HTTP, a TCP port, a DNS name, or a host that answers ping.

## Create one

1. In a project, open **Monitors** and select **New monitor**. The form opens beside the list.
2. Enter a **Name**.
3. Under **Who sees it**, pick **Only your team** or **Also the status page**. See [Who sees it](#who-sees-it).
4. Under **What it checks**, pick **HTTP**, **TCP port**, **DNS name** or **Ping**.
5. Fill in the **Check** section. For HTTP, also fill in **Authentication** and **Headers** as you need.
6. Set what a passing check is under **A check passes when**.
7. Set what a failure does under **Alerts**.
8. Save. The first check runs at a random moment within the first interval. Monitors added together do not check in the same second.

Select **How monitors work** at the top of the page for a short guide with examples.

## TCP, DNS and ping checks

| Kind | Target | Passes when |
| --- | --- | --- |
| TCP port | `host:port`, such as `db.internal:5432`, or `[2001:db8::1]:443` | A connection opens within the timeout. |
| DNS name | A host name, such as `api.example.com` | The name resolves. If you set **The name resolves to**, one A or AAAA answer must equal that address. |
| Ping | A host name or an address, such as `10.0.0.5` | The host answers one ICMP echo request within the timeout. The response time is the round trip. |

The interval, the timeout, the failure count, the alert level and the status page work as for HTTP. A DNS check asks the resolver of the server that runs Monita, so it sees `/etc/hosts` and the server's DNS settings. **Details** shows the same check as a command: `nc -vz` for a port, `getent ahosts` for a name and `ping -c 1` for a host. DNS record types other than A and AAAA are not checked.

A ping check first uses an ICMP socket that needs no root and no extra capability. macOS always allows it. Linux allows it when the sysctl `net.ipv4.ping_group_range` covers Monita's group. systemd 243 and later, and Docker, set it to `0 2147483647`, which covers every group. Where the system refuses the socket, and always on Windows, Monita runs the system `ping` command once instead. If that is missing or refused too, the check fails with an error that names the sysctl. Use a TCP check for a host that blocks ICMP.

## Certificate warning

An HTTP monitor of an `https://` URL can warn before its certificate expires. Under **Alerts**, set **Warn before the certificate expires** to 7, 14 or 30 days.

- Each check reads the expiry date of the server's certificate. **Details** shows it under **Certificate**. When the URL redirects to another host, Monita cannot see the first host's certificate and keeps the last date it read; point the monitor at the address that answers.
- When the date falls inside the window, Monita sends one alert to every alert card that listens to warning. No incident opens.
- Each certificate warns once. A renewed certificate has a new date, so it warns again when its own time comes.
- The monitor's alert switch off sends no warning.

A strict check of an expired certificate fails anyway. The warning gives you time before that happens.

## Request

These fields are for HTTP checks.

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
| Alert level | Two cards, **Critical** and **Warning**. The card picks which alert setup the monitor uses. Each has its own emails and webhook. Default: warning. |
| It counts as down after | The number of failed checks in a row, from 1 to 99. Default: 2. |
| Send alerts to the people in Alerts settings | Off means Monita sends nothing for this monitor. |
| Open an incident | Off means no incident and nothing on the status page. |

With **Open an incident** on, the monitor opens an incident after the set number of failed checks. The first passing check closes it and sends a recovery alert. The cause says "on 1 check" or "after N checks".

With **Open an incident** off, a failure only sends a `[DOWN]` alert, and recovery sends an `[UP]` alert. The team's lists show **Down**. The status page shows nothing. See [Alerts](alerts.md).

A tech lead can also open an incident by hand. See [Incidents](incidents.md).

## Who sees it

**Who sees it** is a choice of two cards under the name. It starts on **Only your team**.

| Card | Means |
| --- | --- |
| **Only your team** | The monitor and its incidents stay inside Monita. |
| **Also the status page** | Visitors see the monitor, its uptime bars and its incidents on the public status page. |

The monitor's incidents reach the status page only when the monitor shows there.

## States

| State | Means |
| --- | --- |
| Up | The last check passed |
| Down | The monitor is past its failed-check count |
| Issue | An open manual incident names it |
| Maintenance | An active maintenance window covers it. The page shows a yellow **In maintenance** stamp under the title, and its list row has yellow stripes. An incident that was open when the window began still shows: the state stays **Down** until a check passes, and the stamp and the yellow stripes still show the window. |
| Paused | A tech lead paused it |
| No data yet | No check has run |

## Pin a monitor

Select the pin at the start of a monitor's row to keep it at the top of the list and on the project's Overview chart, where every pinned monitor gets its own line. Each person pins their own, up to 6 monitors and 6 heartbeats in a project and 6 projects, and the pin on a project's card or row keeps that project first in the project list. Select it again, or Undo in the message, to unpin.

## Pause a monitor

Select **Pause** on the monitor page and confirm. Use it when you take a system out of service and want to keep its history. While a monitor is paused:

- Monita runs no check on it, from this server or from a location.
- No incident opens and no alert goes out.
- The state reads **Paused** in grey, and the page says **Paused since** and the time. A grey **Paused** stamp sits under the title, and its list row has grey stripes.
- The status page hides it.

Pausing closes the monitor's open automatic incident without a resolved alert. The incident's last update reads "Paused by a tech lead." The count of failed checks in a row starts over. Select **Resume** and the next check runs at once.

A maintenance window is different. During a window the checks keep running. See [Maintenance windows](incidents.md#maintenance-windows). A failed check in a window counts as held: the table still shows it, but the chart and the availability count it as good, and the chart draws the window as grey stripes.

## Pause, resume or delete several

Tick the box at the start of each row you want. The box in the heading ticks every monitor the list shows now. A bar with **N selected** replaces the count above the list. A supervisor sees no boxes.

| Button | Does |
| --- | --- |
| **Pause** | Asks first, then pauses every picked monitor. |
| **Resume** | Shows when any picked monitor is paused. It resumes every picked monitor. |
| **Plan maintenance** | Opens the maintenance form with the picked monitors ticked. |
| **Delete** | Asks first. It lists the monitors, what goes and what stays. For more than one, type `delete N` to confirm. **Pause instead** is in the same dialog. |
| **Clear selection** | Unticks every row. |

The action covers up to 100 monitors and it all happens or nothing does. The row menu at the end of a row holds **Edit**, **Pause** or **Resume**, **Plan maintenance** and **Delete** for that monitor alone. **Pause** and **Resume** there offer **Undo** in the message. Deleting a monitor closes its open incident and sends the resolved alert. A script does the same with a `write` API key. See [API](api.md).

## The monitor page

Select **Refresh** to read the latest data. The page also reads itself every 30 seconds.

| Part | Shows |
| --- | --- |
| Stats | How long it has been up or down, the last check, the mean response time, incidents in 30 days |
| Chart | Mean response time per bucket, failed checks in red, down time in a red band, maintenance windows as grey stripes. Hover to read a bucket: the time, the count, and "Up" or "Down for ...". Hour, day and week read the raw checks; month reads the daily summary. |
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
