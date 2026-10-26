# Acceptance Criteria: Sprint 3

## User Stories in Scope

- **US-21** View a status page: Visitor
- **US-22** Read status data as JSON: Project developer
- **US-31** Limit incoming traffic: Super admin
- **US-32** Remove old raw data: Super admin
- **US-33** Back up every night: Super admin
- **US-34** Watch Monita from outside: Super admin

---

## US-21: View a status page (Visitor)

### AC-21.01: Show public items and incidents

```gherkin
Given a project has public monitors and heartbeats
When a visitor opens `/status/<slug>`
Then Monita shows for each public item:
  - the current state
  - a 90-day uptime bar
  And shows incidents with their status updates, newest first
```

### AC-21.02: Hidden items do not appear

```gherkin
Given a monitor is hidden
When a visitor opens the status page
Then the monitor does not appear
```

## US-22: Read status data as JSON (Project developer)

### AC-22.01: Public JSON without a key

```gherkin
Given a project has a status page
When a developer calls the public status API without a key
Then Monita returns the same data as the status page, as JSON
```

## US-31: Limit incoming traffic (Super admin)

### AC-31.01: Limit sign-in per account, never per IP (v1.1 Q45)

```gherkin
Given an account took 10 wrong passwords in the last 15 minutes
When anyone tries to sign in to that account
Then Monita returns 429
And sign-in to other accounts from the same address still works
```

### AC-31.02: Limit per API key

```gherkin
Given an API key sent 120 requests in the last minute
When the key sends another request
Then Monita returns 429
```

### AC-31.03: Change the limits in settings

```gherkin
Given the super admin is signed in
When the super admin opens settings
Then the super admin can change each limit
```

## US-32: Remove old raw data (Super admin)

### AC-32.01: Delete raw checks older than 7 days

```gherkin
Given raw check results are older than 7 days
When the cleanup runs
Then Monita deletes them
```

### AC-32.02: Keep summaries, incidents and tickets

```gherkin
Given the cleanup runs
When it finishes
Then these stay:
  - daily summaries: uptime %, downtime, average latency
  - incidents
  - tickets
```

## US-33: Back up every night (Super admin)

### AC-33.01: Write a nightly backup

```gherkin
Given the super admin set a backup path
When the night passes
Then Monita writes a backup file to that path
```

### AC-33.02: Document the restore

```gherkin
Given a backup file exists
When the super admin reads the install docs
Then the docs show how to restore it
```

## US-34: Watch Monita from outside (Super admin)

### AC-34.01: Health endpoint

```gherkin
Given Monita and its database work
When a caller requests `/healthz`
Then Monita returns 200
```

### AC-34.02: Document the outside heartbeat

```gherkin
Given the super admin reads the install docs
When the super admin looks for outside monitoring
Then the docs show how to set up an outside heartbeat
```

### AC-34.03: Ping a peer every minute

```gherkin
Given the super admin set `MONITA_PING_URL`
When Monita runs
Then Monita sends GET to that URL every 60 seconds
```

> Note: AC-34.01 does not state the response when the database fails. Technical spec 05 section 5.8.1 sets 503.
