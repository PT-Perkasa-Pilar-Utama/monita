# Admin and roles

The **Admin** link in the top menu opens the admin pages. It opens the page you used last. A super admin and a supervisor see this link. A tech lead does not.

## Roles

Monita has three roles.

| Role | Sees | Changes |
| --- | --- | --- |
| Super admin | Every project and every admin page | Everything: projects, users, limits, email, time zone |
| Tech lead | Only the projects they own. Another project answers 404. | Everything inside the projects they own |
| Supervisor | Every project and every admin page | Nothing |

A user has one role. A project has one owner. A tech lead owns the projects they create. A super admin can reassign a project to another tech lead in **Settings**, **General**.

### What a supervisor cannot do or see

A supervisor reads and never writes.

- Every request that changes something answers 403. Only `GET` and `HEAD` work.
- The pages hide the **New**, **Edit**, **Delete** and **Save** controls.
- A heartbeat page shows no ping URL. With it, a supervisor could keep a dead job alive.
- A monitor shows header names but `(hidden)` for the values. It shows `(hidden)` for the request body and the sign-in user name.
- An alert setting shows `(hidden)` for a webhook URL, because the URL works as a credential.
- Secret values never show to anyone.

A project owner cannot become a supervisor. A supervisor is never offered as an owner.

## User management

**Admin**, **User management** lists every account, 20 a page. A super admin does everything here. A supervisor reads the list.

### Invite a user

1. Select **Invite new user**.
2. Enter the email address.
3. Pick a role: **Tech lead** (manages the projects they own), **Supervisor** (sees everything and changes nothing) or **Super admin** (manages every project, the users and the system settings).
4. Select **Send invite**.

Monita emails a sign-up link. The link works once and expires in 72 hours. The page also shows the link with a copy button, in case the email does not arrive. The accounts limit is 500.

### Find and filter

- **Search by email** narrows the list.
- The **Role** filter has **Any role**, **Super admin**, **Supervisor** and **Tech lead**.
- The **Status** filter has **Any status**, **Active**, **Invited** and **Disabled**.
- Click a column head to sort by user, role, status or when the account was added.

### Act on users

Open the menu on a row for one user. Or tick the boxes on several rows. The title bar then shows how many are selected and the actions that fit.

| Action | Does |
| --- | --- |
| Make super admin | The user manages every project, the users and the system settings. |
| Make tech lead | The user keeps the projects they own and loses the admin pages from their next click. |
| Make supervisor | The user sees everything and changes nothing. A user who owns a project cannot become one. |
| Send a new invite | Monita emails a new sign-up link. The password and sessions stop working until the user follows it. |
| Disable | Monita signs the user out now and blocks sign-in. Their projects stay. |
| Enable | The user signs in with the old password. A user who never set one needs a new invite. |

You cannot act on your own account. A disabled user can only be enabled. A batch skips the users it does not fit and says how many. Each action asks you to confirm.

## Rate limits

**Admin**, **Rate limits** holds the limits a super admin can change. They keep Monita fast and stop one visitor or app from overloading it. The defaults suit most teams. Change one only when something hits it. A change applies without a restart.

| Group | Limit | Default |
| --- | --- | --- |
| Monitoring | Checks at the same time | 50 |
| Visitors and apps | Visits per minute from one address. Sign-in attempts count apart, at the same rate. | 60 |
| Visitors and apps | API calls per minute from one key | 120 |
| Visitors and apps | Pings per second for one heartbeat | 1 |
| Bug reports | Bug reports per user per day | 5 |
| Bug reports | Bug reports per project per day | 100 |

Each row shows its default under the box. Select **Save limits** to apply your changes, or **Discard changes** to reset the form.

- **Visits per minute from one address** covers status page visits, bug reports and sign-in tries.
- **Pings per second for one heartbeat**: extra pings in the same second are turned away. The first one still keeps the heartbeat alive.
- **Bug reports per user per day**: Monita knows the user by their ref ID.

Below the form, **Built-in maximums** lists the numbers Monita stops at, with how many are in use. You cannot change them. When a list reaches its maximum, adding one more is refused. See [Limits](limits.md).

## Outgoing email

**Admin**, **Outgoing email** sets the mail server for alerts, digests and invites. It has three sections.

### Mail server

| Field | Means |
| --- | --- |
| Server and Port | The mail server's address and port |
| How Monita connects | **STARTTLS** (usually port 587), **TLS from the start** (usually port 465) or **No encryption** for a relay inside your network (usually port 25). Picking one moves the port to its usual value. |
| User name and Password | Optional. The password never comes back to the page. Leave it blank to keep the saved one. Tick **Remove the saved password** to clear it. |
| Sends from | The sender, for example `Monita <monita@company.co.id>` |
| The server's certificate may be self-signed | Not available with **No encryption** |

The section says which server Monita uses:

1. The server saved on this page.
2. Otherwise `MONITA_SMTP_URL` from the settings file. Saving a server here replaces it.
3. Otherwise none. Monita writes each email to its log.

Select **Test connection** to try the values in the form before you save. It reports "Connected to ... and signed in" or the error. Select **Save changes** to use them. The next email uses the new server. **Remove saved server** goes back to `MONITA_SMTP_URL`. See [Operations](operations.md#email).

Monita stores the password as you enter it, because it must send it to the server.

### Send a test email

Enter **Send it to** (it starts as your own address) and select **Send test email**. Monita sends it now and shows the result on the page: "Sent to ...", the error, or a note that no server is set and the email went to the log. Check the inbox and the spam folder.

### Email delivery

A badge reads **Sending** or **Log only**. The section shows the mail server, the time of the last send, and three counts: **Sent in N days**, **Waiting or retrying** and **Gave up**. After a failure it shows the last error.

## Time zone

**Admin**, **Time zone** sets the offset every page uses to show times. Pick it under **Show times in**, for example `UTC+7: Jakarta (WIB), Bangkok, Hanoi`, and select **Save changes**. The page reloads.

- The offset is fixed. Monita does not follow daylight saving. Change it by hand when your clock changes.
- Charts, "since" times, the Today figure and the 90 days on the status page start at midnight in this offset.
- Monita stores times in UTC. The daily summaries and the month chart still count UTC days. At UTC+7 a month-chart day runs 07:00 to 07:00.
- Ticket daily caps, data retention and the daily digest hour follow UTC.

## System information

**Admin**, **System information** shows how the server is set up, read from its settings file: the mode, the address people open, where it listens, the data file, backups, how long data is kept, and whether it runs behind a proxy. To change a value, edit the settings file and restart Monita. Run `monita whereis` on the server to find the file. See [Configuration](configuration.md).
