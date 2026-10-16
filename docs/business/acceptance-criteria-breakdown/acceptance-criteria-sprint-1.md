# Acceptance Criteria: Sprint 1

## User Stories in Scope

- **US-01** Create the first super admin: Super admin
- **US-02** Create a TL account: Super admin
- **US-03** Set a password from an invite link: TL
- **US-04** Send a new invite link: Super admin
- **US-05** Reassign a project: Super admin
- **US-06** Create a project: TL
- **US-07** Manage API keys: TL
- **US-08** Restrict API keys by IP: TL
- **US-09** Create an HTTP monitor: TL
- **US-10** See recent checks: TL
- **US-11** Spread and cap checks: Super admin
- **US-12** Create a heartbeat: TL
- **US-13** Send a heartbeat ping: Project job
- **US-14** Report a job failure: Project job

---

## US-01: Create the first super admin (Super admin)

### AC-01.01: Create the first super admin from the command line

```gherkin
Given Monita is installed
When the operator runs `monita admin create`
  And enters an email and a password
Then Monita creates a super admin account with that email
```

### AC-01.02: Reject an email that exists

```gherkin
Given an account with the email exists
When the operator runs `monita admin create` with that email
Then the command fails
  And Monita creates no account
```

## US-02: Create a TL account (Super admin)

### AC-02.01: Send an invite link

```gherkin
Given the super admin is signed in
When the super admin creates a TL with an email
Then Monita sends an invite link to that email
```

### AC-02.02: The invite link works once

```gherkin
Given a TL has used an invite link
When the TL opens the same link again
Then the link does not work
```

## US-03: Set a password from an invite link (TL)

### AC-03.01: Open the password form

```gherkin
Given a TL has an unused invite link
When the TL opens the link
Then Monita shows a password form
```

### AC-03.02: Sign in after setting a password

```gherkin
Given the TL is on the password form
When the TL submits a password
Then the TL is signed in
```

### AC-03.03: Reject a used link

```gherkin
Given an invite link was used
When anyone opens the link
Then Monita shows an error
```

## US-04: Send a new invite link (Super admin)

### AC-04.01: Replace the password through a new link

```gherkin
Given a TL forgot the password
When the super admin sends a new invite link
  And the TL sets a password through the new link
Then the new password replaces the old password
```

### AC-04.02: Old links stop working

```gherkin
Given the super admin sent a new invite link to a TL
When the TL opens an older invite link
Then the older link does not work
```

## US-05: Reassign a project (Super admin)

### AC-05.01: The new TL sees the project

```gherkin
Given a project is owned by TL A
When the super admin reassigns the project to TL B
Then TL B sees the project
```

### AC-05.02: The old TL loses the project

```gherkin
Given the super admin reassigned a project from TL A to TL B
When TL A signs in
Then TL A does not see the project
```

### AC-05.03: Project data does not change

```gherkin
Given a project has monitors, incidents and tickets
When the super admin reassigns the project
Then the monitors, incidents and tickets do not change
```

## US-06: Create a project (TL)

### AC-06.01: Create a project

```gherkin
Given the TL is signed in
When the TL creates a project with a name and a slug
Then Monita creates the project
  And the TL is the owner
```

### AC-06.02: The slug is unique

```gherkin
Given a project with the slug exists
When the TL creates a project with the same slug
Then Monita rejects the project
```

### AC-06.03: The slug forms the status page URL

```gherkin
Given a project has the slug "payments"
When anyone opens `/status/payments`
Then Monita shows the status page of that project
```

## US-07: Manage API keys (TL)

### AC-07.01: Create a named key and show it once

```gherkin
Given the TL owns a project
When the TL creates an API key with a name
Then Monita shows the key once
  And later views show the name only
```

### AC-07.02: A revoked key gets 401

```gherkin
Given the TL revoked an API key
When a caller uses that key
Then Monita returns 401
```

## US-08: Restrict API keys by IP (TL)

### AC-08.01: The allowlist is off by default

```gherkin
Given the TL created a project
When the TL opens the project settings
Then the IP allowlist is off
```

### AC-08.02: Block a call from another IP

```gherkin
Given the IP allowlist of a project is on
When a caller uses a valid key from an IP outside the allowlist
Then Monita returns 403
```

## US-09: Create an HTTP monitor (TL)

### AC-09.01: Create a monitor

```gherkin
Given the TL owns a project
When the TL creates a monitor
Then the form has these fields:
  - URL
  - interval: 30s, 1m, 3m or 5m
  - timeout
  - expected status
  - optional keyword
  - severity
  - public flag
```

### AC-09.02: Monitor defaults

```gherkin
Given the TL opens the new monitor form
When the TL leaves the optional fields empty
Then Monita uses these defaults:
  - interval: 1m
  - timeout: 10s
  - expected status: 2xx
  - severity: warning
  - public flag: hidden
```

## US-10: See recent checks (TL)

### AC-10.01: List the last 7 days of checks

```gherkin
Given a monitor has run checks
When the TL opens the monitor
Then Monita lists the checks of the last 7 days with:
  - time
  - status code
  - latency
  - error
```

## US-11: Spread and cap checks (Super admin)

### AC-11.01: Start each monitor at a random offset

```gherkin
Given many monitors have the same interval
When Monita schedules them
Then each monitor starts at a random offset inside its interval
```

### AC-11.02: Run at most 50 checks at once

```gherkin
Given more than 50 checks are due
When Monita runs them
Then no more than 50 checks run at the same time
```

### AC-11.03: Change the values in settings

```gherkin
Given the super admin is signed in
When the super admin opens settings
Then the super admin can change the start offset and the check cap
```

## US-12: Create a heartbeat (TL)

### AC-12.01: Create a heartbeat

```gherkin
Given the TL owns a project
When the TL creates a heartbeat
Then the form has these fields:
  - name
  - period
  - grace time
  - severity
  - public flag
```

### AC-12.02: Show the ping URL

```gherkin
Given the TL created a heartbeat
When the TL opens the heartbeat
Then Monita shows the ping URL `/ping/<token>`
```

## US-13: Send a heartbeat ping (Project job)

### AC-13.01: GET, HEAD and POST count as a ping

```gherkin
Given a heartbeat exists
When the project job sends GET, HEAD or POST to `/ping/<token>`
Then Monita records a ping
```

### AC-13.02: Limit pings to 1 per second

```gherkin
Given the project job sent a ping in the last second
When the project job sends another ping
Then Monita returns 429
  And the heartbeat still counts as alive
```

## US-14: Report a job failure (Project job)

### AC-14.01: Open an incident at once

```gherkin
Given a heartbeat exists
When the project job sends a request to `/ping/<token>/fail`
Then Monita opens an incident without waiting for the grace time
```

> Note: AC-14.01 depends on incidents, which arrive in Sprint 2. In Sprint 1, the failure is stored as the heartbeat state.
> Note: AC-11.03: the start offset is random per monitor and has no setting. The limits page sets the check cap only.
