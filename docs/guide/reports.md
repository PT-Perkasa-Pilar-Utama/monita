# Reports

Monita makes two kinds of report as PDF files: a summary of a project over a range of days, and the report of one incident. Give them to a manager or a client who does not use Monita.

Every role that can open the project can download its reports, a supervisor included. A report shows the times in the admin time zone (**Admin**, **Time zone**) and names the offset on its first line.

## Download a summary report

1. Open the project's **Overview**.
2. Select **Download report**.
3. Select **Last 7 days**, **Last month** or **Last quarter**. Monita downloads the file.

To pick the days yourself:

1. Select **Download report**, then **Pick the days**.
2. Enter the **First day** and the **Last day**. The report covers both days and every day between them, up to 366 days.
3. Select **Download**.

The last 7 days end yesterday. The last month is the calendar month before this one, and the last quarter is the three calendar months before this quarter (January to March, April to June, July to September or October to December).

## What a summary report shows

- The project's availability, its downtime, the number of incidents and the mean time to resolve.
- A row for each monitor and heartbeat: availability, downtime, incidents and the longest incident.
- The incidents that were open in the range, longest first. The table shows 50 at most and says when there are more.
- The maintenance windows that overlap the range.

Availability counts automatic incidents only, as the status page and the monitor page do. A manual incident, an incident reclassified as planned work or a false alarm, and time in a maintenance window are not downtime. The project's availability adds up the time of all its systems, so a system that existed for half the range counts half. The mean time to resolve is the mean time from start to resolve of the counted incidents that were resolved in the range.

A report never shows a monitor's URL, so a token in a URL stays private.

## Download an incident report

1. Open the incident.
2. Select **Download report**.

The report shows the alert level, who opened the incident, when it started and was resolved, its class, the affected monitors and heartbeats, the cause Monita recorded, and every update oldest first with who posted it.

## Limits

- A report prints letters A to Z, digits and the usual punctuation. Any other character, such as `é` or `✓`, prints as `?`.
- Response times are not in a report.
