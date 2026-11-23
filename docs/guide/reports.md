# Reports

Monita makes two kinds of report as PDF files: a summary of a project over a range of days, and the report of one incident. Give them to a manager or a client who does not use Monita.

Every role that can open the project can download its reports, a supervisor included. A downloaded report shows times on your own computer's clock, as the team's pages do, and names the offset on its first line. An emailed report uses the admin time zone (**Admin**, **Time zone**).

## Download a summary report

1. Open the project's **Overview**.
2. Select **Download report**.
3. Select **Last 7 days**, **Last month** or **Last quarter**. Monita downloads the file.

To pick the days yourself:

1. Select **Download report**, then **Pick the days**.
2. Enter the **First day** and the **Last day**. The report covers both days and every day between them, up to 366 days.
3. Select **Download**.

The last 7 days end yesterday, on your clock. The last month is the calendar month before this one, and the last quarter is the three calendar months before this quarter (January to March, April to June, July to September or October to December).

## What a summary report shows

- Four figures: the project's availability, its downtime, the number of outages and the mean time to resolve.
- Availability by day: one bar a day, green with no downtime, yellow when down for less than 1% of the day, red for more, and grey before the first monitor or heartbeat.
- A row for each monitor and heartbeat: availability, downtime, outages, MTTR (the mean length of an outage) and MTBF (the time up divided by the outages).
- The maintenance time and its share of the period, with the windows that overlap it.
- Each incident that was open in the range, longest first, manual and reclassified ones too: its alert level and state, when it started and ended, what it affects, its cause and its latest update. Monita reads the newest 200 incidents and shows the 50 longest, and the report says when there are more.

An outage is an incident that Monita opened after failed checks and nobody reclassified as planned work or a false alarm. Availability counts outages only, as the status page and the monitor page do, so a manual incident and time in a maintenance window are not downtime. The project's availability adds up the time of all its systems, so a system that existed for half the range counts half. The mean time to resolve is the mean time from start to resolve of the outages resolved in the range.

A report never shows a monitor's URL, so a token in a URL stays private.

## Download an incident report

1. Open the incident.
2. Select **Download report**.

The report shows the alert level, who opened the incident, when it started and was resolved, its class, the affected monitors and heartbeats, the cause Monita recorded, and the updates oldest first with who posted them. An incident takes 50 updates at most. The incident report is for the team: the cause can name internal hosts, and each update names its author. For people outside the team, link the status page, which shows neither.

## Send a report every week or month

A project can email its summary report as a PDF every week, every month, or both. The weekly report covers Monday to Sunday and the monthly report the calendar month, in the admin time zone. Monita sends each one once, at the hour of the daily digest (`MONITA_DIGEST_UTC_HOUR`) after its period ends.

1. In the project, open **Settings**, **Reports**.
2. Under **Weekly report** or **Monthly report**, turn on **Send this report**.
3. Under **Email addresses**, enter one address a line, up to 20.
4. Select **Save changes**.

The first email goes out after the current period ends: a weekly report turned on on a Wednesday first covers the week that holds that Wednesday, and a monthly report turned on today first covers this month. To stop the emails, turn off **Send this report** and select **Save changes**. The email names the project and the period and links to the project. Monita sends it through the mail server set in **Admin**, **Outgoing email**.

The project owner and a super admin change the schedules. A supervisor reads them.

The outbox tries a report email again after 1, 5 and 15 minutes. When every try fails, that period is not sent again, so download it from **Overview** instead. **Admin**, **Outgoing email** counts the emails that failed. With no mail server set, Monita writes the email to its log and counts the period as sent.

## Limits

- A report prints letters A to Z, digits and the usual punctuation, and turns curly quotes, dashes and the ellipsis into their plain forms. Any other character, such as `é` or `✓`, prints as `?`.
- Response times are not in a report.
