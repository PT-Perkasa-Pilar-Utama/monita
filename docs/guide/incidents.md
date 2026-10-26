# Incidents

An incident records a problem from start to end, sends alerts, and tells users on the status page. Alert settings have their own page: [Alerts](alerts.md).

## Automatic incidents

| Source | Opens | Closes |
| --- | --- | --- |
| Monitor | The monitor's number of failed checks in a row (1 to 99, default 2) | The first passing check |
| Heartbeat | A missed deadline, or a call to `/fail` | The next ping |

A monitor or heartbeat with **Open an incident** off opens nothing. It sends a `[DOWN]` and an `[UP]` alert instead. See [Alerts](alerts.md#alerts-without-an-incident).

Monita posts the first update, **Investigating**, with the cause. A tech lead can add updates while the incident is open.

## The incident page

The page shows the updates as a thread, newest first. Monita's own updates read "Monita". A sidebar shows:

- **Affects**: the monitors and heartbeats.
- **Cause**: the technical cause Monita recorded. The status page never shows it.
- **Dates**: when it started, when it ended and how long it lasted.
- **How it closes**: what ends this incident.

## Post an update

Select **Post update**. A side panel opens.

1. Pick a **Label**.
2. Edit the **Message**. It starts from a template for the label.
3. Select **Post update**.

The update appears on the status page at once, without your name.

| Label | Means | Starting message |
| --- | --- | --- |
| Investigating | Something is wrong and you are looking into it | We are looking into the problem and will post an update when we know more. |
| Identified | You found the cause and are working on a fix | We found the cause and are working on a fix, and we will post an update when it is out. |
| Monitoring | A fix is out and you are watching to be sure it holds | A fix is out and we are watching to make sure it holds. |
| Resolved | It is over | The problem is fixed and everything works again. |

The panel opens on **Monitoring**. When you pick another label, the message changes to that label's template. Your own text stays. Reopening the panel resets the label and the message.

A closed incident takes only follow-up notes labelled **Resolved**. An incident has at most 50 updates.

## Resolve an automatic incident by hand

Post **Resolved** on an open automatic incident to close it yourself, for example when you retire a job. Monita closes it now and sends the resolved alert. The panel says "marked resolved by a tech lead".

The source then starts over:

- A monitor needs its full count of new failed checks to open the next incident.
- A heartbeat gets a new deadline: now plus **Runs every** plus **Can be late by**. A job that stays silent opens a new incident at that time.

## Manual incidents

Use **Report incident** for a problem no check catches, such as slow sign-in for some users. Report one too when a monitor with incidents off goes down and users should know.

1. Open **Incidents** in a project and select **Report incident**. Select **When to report an incident** for a short guide.
2. Enter a **Title** and pick an **Alert level**.
3. Tick the monitors and heartbeats it affects. **Find a monitor or heartbeat by name** narrows the list.
4. Write the first **Message**.

A manual incident marks its items as **Issue**, not down. It never counts as downtime. It closes only when a tech lead posts **Resolved**. A project holds at most 20 open manual incidents. A manual incident sends no alert.

## Reclassify an incident

Some outages do not count: the work was planned, or the check was wrong. A tech lead can mark the incident so it leaves the numbers. Open the incident and select **Reclassify**. Pick one:

| Choice | Use it when |
| --- | --- |
| **Planned work** | The outage was maintenance, or a pause that came too late. |
| **False alarm** | The check was wrong and nothing was down. |
| **Count it again** | You marked it by mistake. The incident counts again. |

Each choice opens a dialog that says what changes. After you confirm:

- An open incident closes now with a **Resolved** update. An automatic one also sends the resolved alert.
- The incident leaves uptime, the availability figures, the charts and the status page, with its history.
- The team still sees the incident. Its page shows the class and who marked it. The lists show a **Planned work** or **False alarm** tag.
- **Count it again** does not reopen the incident. It stays closed and counts in the numbers again.
- Past days change too, because Monita works out downtime from the incidents each time a page loads.

A supervisor sees the class and cannot change it. A `write` API key can reclassify too. See [API](api.md).

## Maintenance windows

Plan a window before planned work, so the work neither alerts the team nor shows as an outage.

1. Open **Maintenance** in a project and select **Plan maintenance**. Or open the row menu of a monitor or heartbeat and select **Plan maintenance**. Or tick several rows and select **Plan maintenance** in the bar above the list. The form opens with those systems already ticked.
2. Enter a **Title**, then pick when it **Starts** and **Ends**. Type the times on your own clock. A line under them says when the window starts and how long it lasts. It also shows how Monita displays the window when Monita's time zone differs from your clock. A start in the past starts now. A window lasts 30 days at most.
3. Tick the monitors and heartbeats it covers.
4. Optional: write a **Message on the status page**.

While a window is active:

- Monita still checks the items and stores every result.
- A failure opens no incident and sends no alert.
- A failed check or ping counts as held. It stays in the checks and pings tables, but the charts, the response-time sparklines and the daily summary count it as good, so planned work leaves no red. The monitor and heartbeat charts draw the window as grey stripes. Hover a stripe to read the window's title and length.
- A heartbeat's `/fail` ping in the window opens nothing then. If the job still has not sent a success ping when the window ends, the incident opens then, with the failure text the job sent.
- The items show **Maintenance** on the team's pages and on the status page. The status page shows the title, the times and the message.
- A window adds no downtime. An incident that was open before the window stays open, closes as usual, and counts as downtime. Until then the item reads **Down**, but its list row still has the yellow stripes and its page still shows the **In maintenance** stamp.
- A paused item in a window stays paused: Monita checks nothing, so the window has nothing to hold. Resume it inside the window, and the window holds its failures from then on. Pause an item in a window, and Monita stops watching it at once, as any pause does.

When the window ends, a check that still fails opens an incident at once. A heartbeat that stayed silent opens one when the window ends.

| State | What you can do |
| --- | --- |
| Scheduled | **Edit** any field, or **Delete** it. |
| Active | **Edit** the title, the message and the end, or **End now**. Failures open incidents again from that moment. |
| Ended | Read it. It cannot change. |

An active window keeps its start and its items, because the checks it already holds count as maintenance. To cover one more system, plan a second window for it.

A project has at most 20 windows scheduled or active.

The **Maintenance** page lists the windows 20 a page. Each row names the systems it covers, with links to their pages, and its message. **Search windows** finds a title. The state filter shows **Scheduled**, **Active** or **Ended**. Pick a first and a last day to see the windows that overlap those days, on your own clock. Leave either one empty for no bound. Select **When** to sort by start, newest first or oldest first. Below the list, a second card shows the count of paused systems. It lists every paused monitor and heartbeat with a **Resume** button. The number on the **Maintenance** tab counts the monitors and heartbeats that are paused or in an active window now, each once.

The old address `/projects/<project>/incidents/maintenance` still opens this page.

## The Incidents page

A project's **Incidents** page lists its incidents with alert level, latest update, start and duration. Filter by **Open**, **Resolved** or **All**. Search by title and filter by severity. A reclassified incident shows its **Planned work** or **False alarm** tag under the title.

### Across projects

**Incidents** in the top menu lists the incidents of every project you can open. A tech lead sees the projects they own. A super admin and a supervisor see all.

- The list shows open incidents first. Critical ones come first.
- Use the state filter for **Open**, **Resolved** or **Open and resolved**. Use **Search incidents** and the alert level filter to narrow the list.
- Sort by **Alert level** or **Started**.
- A reclassified incident shows its **Planned work** or **False alarm** tag under the title.
- The list shows 25 a page. A count on the menu shows the open incidents.
- A row opens the incident. The crumb **All incidents** returns to the same search and page.

## Daily digest

Once a day, at `MONITA_DIGEST_UTC_HOUR`, each project owner gets one email listing new tickets still in Triage. No email goes out when there are none. The digest is off for a new project. Turn on the **Daily digest** card in [Ticket alerts](alerts.md#ticket-alerts).
