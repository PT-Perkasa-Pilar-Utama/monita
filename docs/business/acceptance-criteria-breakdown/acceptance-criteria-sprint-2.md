# Acceptance Criteria: Sprint 2

## User Stories in Scope

- **US-15** Set alert recipients per severity: TL
- **US-16** Get alerts on open and close: TL
- **US-17** Open incidents automatically: TL
- **US-18** Close incidents automatically: TL
- **US-19** Post a status update: TL
- **US-20** Open an incident by hand: TL

---

## US-15: Set alert recipients per severity (TL)

### AC-15.01: Alert cards (v1.1 Q34)

```gherkin
Given the TL owns a project
When the TL opens the alert setup
Then the TL sees the project's alert cards
  And can add a card: an email list, or a webhook in one format
  And each card listens to critical, warning or both
  And each card has its own on and off switch and its own test
```

### AC-15.02: Default routing

```gherkin
Given the TL has not changed the alert setup
When an alert fires
Then it goes to the owner's email card for its level
  And no webhook is called until the TL adds a webhook card

Given no card that is on listens to a level
When an alert of that level fires
Then no one is alerted
  And the incident still opens and shows
  And the alerts page says that no card listens to that level
```

## US-16: Get alerts on open and close (TL)

### AC-16.01: Alert on open

```gherkin
Given a monitor or heartbeat has alert recipients
When an incident opens
Then Monita sends one alert with:
  - the project
  - the monitor or heartbeat
  - the severity
  - the type: down or heartbeat missed
  - the error
```

### AC-16.02: Alert on close

```gherkin
Given an incident is open
When the incident closes
Then Monita sends one alert with the type "recovered"
```

## US-17: Open incidents automatically (TL)

### AC-17.01: Two failed checks open an incident

```gherkin
Given a monitor has no open incident
When 2 checks in a row fail
Then Monita opens an incident
```

### AC-17.02: One failed check does not open an incident

```gherkin
Given a monitor has no open incident
When 1 check fails
  And the next check passes
Then Monita opens no incident
```

### AC-17.03: A missed heartbeat opens an incident

```gherkin
Given a heartbeat has a period and a grace time
When no ping arrives within the period plus the grace time
Then Monita opens an incident
```

## US-18: Close incidents automatically (TL)

### AC-18.01: The first pass closes the incident

```gherkin
Given an automatic incident is open
When the first check passes or the first ping arrives
Then Monita closes the incident
  And sets the label Resolved
```

### AC-18.02: Show start, end and duration

```gherkin
Given an incident is closed
When the TL opens the incident
Then Monita shows:
  - start
  - end
  - duration
```

## US-19: Post a status update (TL)

### AC-19.01: Post an update with a label

```gherkin
Given an incident exists
When the TL posts a status update
Then the TL picks one label:
  - Investigating
  - Identified
  - Monitoring
  - Resolved
```

### AC-19.02: The message is required

```gherkin
Given the TL writes a status update
When the TL submits it without a message
Then Monita rejects the update
```

### AC-19.03: Post after the incident is closed

```gherkin
Given an incident is closed
When the TL posts a status update with the label Resolved
Then Monita adds the update to the incident
  And Monita refuses any other label
```

## US-20: Open an incident by hand (TL)

### AC-20.01: Open a manual incident

```gherkin
Given the TL owns a project
When the TL opens an incident by hand
Then the TL picks the affected monitors or heartbeats
  And writes a first message
```

### AC-20.02: Resolved closes a manual incident

```gherkin
Given a manual incident is open
When the TL posts a status update with the label Resolved
Then Monita closes the incident
```
