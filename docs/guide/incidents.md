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
| Investigating | Something is wrong and you are looking into it | We are looking into the problem. We will post an update when we know more. |
| Identified | You found the cause and are working on a fix | We found the cause and are working on a fix. We will post an update when it is out. |
| Monitoring | A fix is out. You are watching to be sure it holds. | A fix is out. We are watching to make sure it holds. |
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

## Maintenance windows

Plan a window before planned work, so the work neither alerts the team nor shows as an outage.

1. Open **Incidents** in a project and select **Maintenance**, then **Plan maintenance**. Or select **Plan maintenance** on a monitor or heartbeat page. The form opens with that system already ticked.
2. Enter a **Title**, then pick when it **Starts** and **Ends**. Times are in the time zone a super admin set. A start in the past starts now. A window lasts 30 days at most.
3. Tick the monitors and heartbeats it covers.
4. Optional: write a **Message on the status page**.

While a window is active:

- Monita still checks the items and stores every result.
- A failure opens no incident and sends no alert.
- A heartbeat's `/fail` ping in the window opens nothing then. If the job still has not sent a success ping when the window ends, the incident opens then, with the failure text the job sent.
- The items show **Maintenance** on the team's pages and on the status page. The status page shows the title, the times and the message.
- A window adds no downtime. An incident that was open before the window stays open, closes as usual, and counts as downtime.

When the window ends, a check that still fails opens an incident at once. A heartbeat that stayed silent opens one when the window ends.

| State | What you can do |
| --- | --- |
| Scheduled | **Delete** it. |
| Active | **End now**. Failures open incidents again from that moment. |
| Ended | Read it. It cannot change. |

A project has at most 20 windows scheduled or active. A window cannot be edited: delete it and plan a new one.

## The Incidents page

A project's **Incidents** page lists its incidents with alert level, latest update, start and duration. Filter by **Open**, **Resolved** or **All**. Search by title and filter by severity.

### Across projects

**Incidents** in the top menu lists the incidents of every project you can open. A tech lead sees the projects they own. A super admin and a supervisor see all.

- The list shows open incidents first. Critical ones come first.
- Use the state filter for **Open**, **Resolved** or **Open and resolved**. Use **Search incidents** and the alert level filter to narrow the list.
- Sort by **Alert level** or **Started**.
- The list shows 25 a page. A count on the menu shows the open incidents.
- A row opens the incident. The crumb **All incidents** returns to the same search and page.

## Daily digest

Once a day, at `MONITA_DIGEST_UTC_HOUR`, each project owner gets one email listing new tickets still in Triage. No email goes out when there are none. The digest is off for a new project. Turn on the **Daily digest** card in [Ticket alerts](alerts.md#ticket-alerts).
