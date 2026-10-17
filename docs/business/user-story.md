# Monita v1: User Stories

Source: [decision log 2026-10-16](../decisions/2026-10-16-monita-v1-scope.md).

## Personas

| Persona | Who |
|---------|-----|
| Super admin | Runs Monita for the IT department. |
| TL | Tech lead of a project. Owns the project in Monita. |
| Project backend | The server of a project. Calls Monita with an API key. |
| Project job | A cron job or worker that sends heartbeat pings. |
| Project developer | Builds a custom status page from the Monita API. |
| Reporter | An end user of a project who submits a ticket. |
| Visitor | Anyone who opens a public page. No sign-in. |

### US-01: Create the first super admin

- **Persona:** Super admin
- **Action:** Create the first account from the command line.
- **Business value:** A new install can be signed in to.

### US-02: Create a TL account

- **Persona:** Super admin
- **Action:** Create a TL account that gets an invite link by email.
- **Business value:** A TL can use Monita.

### US-03: Set a password from an invite link

- **Persona:** TL
- **Action:** Set a password from the invite link.
- **Business value:** The TL can sign in.

### US-04: Send a new invite link

- **Persona:** Super admin
- **Action:** Send a new invite link to a TL.
- **Business value:** A TL who forgot the password can get back in.

### US-05: Reassign a project

- **Persona:** Super admin
- **Action:** Reassign a project to another TL.
- **Business value:** A project keeps an owner when a TL leaves.

### US-06: Create a project

- **Persona:** TL
- **Action:** Create a project with a name. Monita makes its slug.
- **Business value:** Monitors, heartbeats and tickets are grouped per project.

### US-07: Manage API keys

- **Persona:** TL
- **Action:** Create, name and revoke API keys.
- **Business value:** The project backend can call Monita.

### US-08: Restrict API keys by IP

- **Persona:** TL
- **Action:** Set an optional IP allowlist for the project API keys.
- **Business value:** A stolen key is useless outside the project servers.

### US-09: Create an HTTP monitor

- **Persona:** TL
- **Action:** Create an HTTP monitor.
- **Business value:** Monita checks the project URL.

### US-10: See recent checks

- **Persona:** TL
- **Action:** See the recent checks of each monitor.
- **Business value:** The TL sees when and why a monitor failed.

### US-11: Spread and cap checks

- **Persona:** Super admin
- **Action:** Spread checks over time and cap how many run at once.
- **Business value:** Many monitors do not overload Monita.

### US-12: Create a heartbeat

- **Persona:** TL
- **Action:** Create a heartbeat.
- **Business value:** The TL knows when a job stops running.

### US-13: Send a heartbeat ping

- **Persona:** Project job
- **Action:** Ping the heartbeat URL.
- **Business value:** Monita knows the job ran.

### US-14: Report a job failure

- **Persona:** Project job
- **Action:** Report a failure to the heartbeat.
- **Business value:** The TL knows at once.

### US-15: Set alert recipients per severity

- **Persona:** TL
- **Action:** Set who gets alerts for each severity.
- **Business value:** Critical problems reach chat, and minor ones do not.

### US-16: Get alerts on open and close

- **Persona:** TL
- **Action:** Get an alert when an incident opens and when it closes.
- **Business value:** The TL knows about problems and recoveries.

### US-17: Open incidents automatically

- **Persona:** TL
- **Action:** Let Monita open an incident by itself.
- **Business value:** The TL does not miss a failure.

### US-18: Close incidents automatically

- **Persona:** TL
- **Action:** Let Monita close an incident when the check passes.
- **Business value:** The downtime figure is exact.

### US-19: Post a status update

- **Persona:** TL
- **Action:** Post a status update on an incident.
- **Business value:** People know what is happening.

### US-20: Open an incident by hand

- **Persona:** TL
- **Action:** Open an incident by hand.
- **Business value:** The TL can report a problem that no check catches.

### US-21: View a status page

- **Persona:** Visitor
- **Action:** View the status page of a project.
- **Business value:** The visitor sees whether the project works.

### US-22: Read status data as JSON

- **Persona:** Project developer
- **Action:** Read the status data as JSON.
- **Business value:** The project can build its own status page.

### US-23: Get a ref ID

- **Persona:** Project backend
- **Action:** Get a ref ID from Monita.
- **Business value:** A project user can submit a ticket.

### US-24: Submit a ticket

- **Persona:** Reporter
- **Action:** Submit a ticket with a ref ID.
- **Business value:** The team knows about a bug.

### US-25: Move tickets on a kanban

- **Persona:** TL
- **Action:** Move tickets on a kanban.
- **Business value:** The TL tracks the work.

### US-26: Rewrite a public title

- **Persona:** TL
- **Action:** Rewrite the public title of a ticket.
- **Business value:** Personal data does not go public.

### US-27: Revoke a ref ID

- **Persona:** TL
- **Action:** Revoke a ref ID.
- **Business value:** An abusive reporter cannot submit more tickets.

### US-28: View the public kanban

- **Persona:** Visitor
- **Action:** View the public kanban of a project.
- **Business value:** The visitor sees known bugs and their state.

### US-29: See my own tickets

- **Persona:** Reporter
- **Action:** See my own tickets.
- **Business value:** The reporter can follow their state.

### US-30: Hear about new tickets

- **Persona:** TL
- **Action:** Get told about new tickets.
- **Business value:** The TL can triage them.

### US-31: Limit incoming traffic

- **Persona:** Super admin
- **Action:** Limit incoming traffic.
- **Business value:** One bad caller cannot take Monita down.

### US-32: Remove old raw data

- **Persona:** Super admin
- **Action:** Remove old raw check data.
- **Business value:** The database stays small.

### US-33: Back up every night

- **Persona:** Super admin
- **Action:** Get a backup every night.
- **Business value:** Monita can be restored after a disk failure.

### US-34: Watch Monita from outside

- **Persona:** Super admin
- **Action:** Let an outside service watch Monita.
- **Business value:** The super admin knows when Monita itself is down.
