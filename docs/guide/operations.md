# Operations

Keep Monita healthy: back it up, restore it, watch it from outside, and upgrade it.

## Backups and restore

Monita writes `monita-YYYYMMDD.db` to `MONITA_BACKUP_DIR` at 01:00 UTC and keeps the newest `MONITA_BACKUP_KEEP` (7). To restore one:

```sh
systemctl stop monita
mv /var/lib/monita/monita.db /var/lib/monita/monita.db.broken
rm -f /var/lib/monita/monita.db-wal /var/lib/monita/monita.db-shm
install -m 0600 -o monita /var/backups/monita/monita-20261016.db /var/lib/monita/monita.db
systemctl start monita
```

A backup is a plain SQLite file: `sqlite3 monita-20261016.db 'PRAGMA integrity_check'` checks it.

## Watch Monita from outside

Monita cannot alert about its own outage. Run a second Monita at another site and give each a heartbeat on the other:

1. On Monita B, create a heartbeat named `monita-a`, runs every 1 minute, can be late by 5 minutes, severity critical.
2. On Monita A, set `MONITA_PING_URL` to that heartbeat's ping URL and restart. A pings it at start and every 60 seconds.
3. Do the same in the other direction.

If one site loses power, the other sends "monita-a missed its heartbeat". A free Better Stack heartbeat works the same way in place of Monita B. `GET /healthz` returns 503 when the database fails or the scheduler stops ticking, for a load balancer or an uptime check.

## Email

Monita picks its mail server in this order:

1. The server a super admin saved on **Admin**, **Outgoing email**.
2. The `MONITA_SMTP_URL` variable in the env file.
3. None. Monita writes each email to its log.

So `MONITA_SMTP_URL` is a fallback. Saving a server on the page replaces it for the next email, with no restart. **Remove saved server** returns to the variable.

Use **Test connection** to check a server before you save it. Use **Send a test email** to see a real delivery. The page also counts emails sent, waiting and failed. See [Admin and roles](admin.md#outgoing-email).

The saved password is stored as you entered it. Only the database file holds it. Back up that file with care.

## Lost super admin password

```sh
systemctl stop monita        # the running service locks the database file
sudo monita admin reset-password --email you@company.co.id
systemctl start monita
```

The command asks for the new password without echo (or reads stdin in a script) and signs the admin out everywhere. A tech lead gets a new invite from a super admin instead.

## Upgrade

Take a backup, install the new binary over the old one, and restart. Migrations run at start.

```sh
systemctl stop monita && cp /var/lib/monita/monita.db /var/backups/monita/pre-upgrade.db
install -m 0755 monita /usr/local/bin/monita
systemctl start monita
```

## Cut a release (maintainers)

Push a `v*` tag that matches the version in `Cargo.toml`. [release.yml](../../.github/workflows/release.yml) builds the static Linux binary, boots it, and opens a draft GitHub release with `checksums.txt`.
