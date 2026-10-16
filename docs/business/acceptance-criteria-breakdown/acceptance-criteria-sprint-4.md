# Acceptance Criteria: Sprint 4

## User Stories in Scope

- **US-23** Get a ref ID: Project backend
- **US-24** Submit a ticket: Reporter
- **US-25** Move tickets on a kanban: TL
- **US-26** Rewrite a public title: TL
- **US-27** Revoke a ref ID: TL
- **US-28** View the public kanban: Visitor
- **US-29** See my own tickets: Reporter
- **US-30** Hear about new tickets: TL

---

## US-23: Get a ref ID (Project backend)

### AC-23.01: Get a reusable reporter ID

```gherkin
Given the project backend has a valid API key
When it calls `POST /api/v1/ref-ids` with `single_use: false`
Then Monita returns a reporter ID that many tickets can use
```

### AC-23.02: Get a single-use ID

```gherkin
Given the project backend has a valid API key
When it calls `POST /api/v1/ref-ids` with `single_use: true`
Then Monita returns an ID that one ticket uses up
```

### AC-23.03: Reject a missing or bad key

```gherkin
Given a caller has no API key or a bad API key
When it calls `POST /api/v1/ref-ids`
Then Monita returns 401
```

## US-24: Submit a ticket (Reporter)

### AC-24.01: Submit through the Monita form

```gherkin
Given a reporter has a valid ref ID
When the reporter submits the Monita ticket form
Then Monita creates a ticket with:
  - ref ID
  - title
  - description
  - optional page URL
```

### AC-24.02: Submit through the API

```gherkin
Given a reporter has a valid ref ID
When the project UI submits the ticket through the API
Then Monita creates the ticket
```

### AC-24.03: Reject a bad ref ID

```gherkin
Given a ref ID is unknown, revoked or used up
When a ticket is submitted with it
Then Monita returns an error
```

### AC-24.04: Limit tickets per ref ID

```gherkin
Given a ref ID submitted 5 tickets today
When it submits another ticket
Then Monita returns 429
```

### AC-24.05: Limit tickets per project

```gherkin
Given a project received 100 tickets today
When another ticket is submitted
Then Monita returns 429
```

## US-25: Move tickets on a kanban (TL)

### AC-25.01: Kanban columns

```gherkin
Given the TL owns a project
When the TL opens the kanban
Then the columns are:
  - Triage
  - Backlog
  - In progress
  - Done
```

### AC-25.02: New tickets land in Triage

```gherkin
Given a reporter submits a ticket
When Monita creates it
Then the ticket is in Triage
```

### AC-25.03: Done needs a close reason

```gherkin
Given the TL moves a ticket to Done
When the TL confirms
Then the TL picks one close reason:
  - Fixed
  - Won't fix
  - Duplicate
```

## US-26: Rewrite a public title (TL)

### AC-26.01: Show the public title only

```gherkin
Given the TL rewrote the public title of a ticket
When a visitor opens the public kanban
Then the visitor sees the public title
  And the original title stays private
```

## US-27: Revoke a ref ID (TL)

### AC-27.01: A revoked ref ID gets an error

```gherkin
Given the TL revoked a ref ID
When a ticket is submitted with it
Then Monita returns an error
```

### AC-27.02: Existing tickets stay

```gherkin
Given a ref ID has tickets
When the TL revokes the ref ID
Then the tickets stay
```

## US-28: View the public kanban (Visitor)

### AC-28.01: Hide Triage

```gherkin
Given a project has tickets in Triage and in other columns
When a visitor opens the public kanban
Then the visitor sees only tickets outside Triage
```

### AC-28.02: Public fields only

```gherkin
Given a ticket is public
When a visitor views it
Then the visitor sees:
  - title
  - column
  - created date
  - last update
  And the visitor does not see the description or the reporter
```

## US-29: See my own tickets (Reporter)

### AC-29.01: List tickets by ref ID

```gherkin
Given a reporter has a ref ID with tickets
When the reporter enters the ref ID
Then Monita lists the reporter's tickets, including tickets in Triage
```

## US-30: Hear about new tickets (TL)

### AC-30.01: Daily digest

```gherkin
Given a project received new tickets in Triage today
When the daily digest runs
Then Monita sends one email that lists them
```

### AC-30.02: No digest without new tickets

```gherkin
Given a project received no new tickets
When the daily digest runs
Then Monita sends no email
```

### AC-30.03: Webhook for each new ticket

```gherkin
Given the project has a warning-level webhook
When a reporter submits a ticket
Then Monita calls the webhook for that ticket
```
