# Install and run Monita

Monita runs as one systemd service on a Linux VM. These steps assume Ubuntu or Debian and root access.

## Install

1. Download the archive from [Releases](https://github.com/PT-Perkasa-Pilar-Utama/monita/releases), or build it with `make release`. Check it against `checksums.txt`, then install it:

   ```sh
   sha256sum -c checksums.txt --ignore-missing
   tar -xzf monita_v0.1.0_linux_amd64.tar.gz
   install -m 0755 monita /usr/local/bin/monita
   ```

2. Answer the setup questions. Setup writes `/etc/monita/monita.env` (mode 0600) and creates the first super admin:

   ```sh
   sudo monita setup
   ```

   [Spec 11](technical-specs/11-environment-configuration.md) describes every variable. For a scripted install, `monita setup --yes` takes the answers from `MONITA_*` variables ([the CLI contract](api-specs/10-cli.md)).

3. Install and start the service. It creates the `monita` user and the folders, writes the systemd unit, and waits for `/healthz`:

   ```sh
   sudo monita service install
   monita whereis               # env file, database, backups, unit, logs
   ```

4. Put the company reverse proxy in front for HTTPS. Setup's defaults, `MONITA_TRUST_PROXY=true` and `MONITA_BIND=127.0.0.1:8080`, assume one. If other hosts can reach the port, they can forge `X-Forwarded-For`.

`monita service status`, `monita service logs -f` and `monita service uninstall` cover the rest. `service install` writes the same unit as [deploy/monita.service](../deploy/monita.service), with this host's paths.

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

1. On Monita B, create a heartbeat named `monita-a`, period 1 minute, grace 5 minutes, severity critical.
2. On Monita A, set `MONITA_PING_URL` to that heartbeat's ping URL and restart. A pings it at start and every 60 seconds.
3. Do the same in the other direction.

If one site loses power, the other sends "monita-a missed its heartbeat". A free Better Stack heartbeat works the same way in place of Monita B. `GET /healthz` returns 503 when the database fails or the scheduler stops ticking, for a load balancer or an uptime check.

## Lost super admin password

```sh
systemctl stop monita        # the running service locks the database file
sudo monita admin reset-password --email you@company.co.id
systemctl start monita
```

The command asks for the new password without echo (or reads stdin in a script) and signs the admin out everywhere. A TL gets a new invite from a super admin instead.

## Upgrade

Take a backup, install the new binary over the old one, and restart. Migrations run at start.

```sh
systemctl stop monita && cp /var/lib/monita/monita.db /var/backups/monita/pre-upgrade.db
install -m 0755 monita /usr/local/bin/monita
systemctl start monita
```

## Cut a release

Push a `v*` tag that matches the version in `Cargo.toml`. [release.yml](../.github/workflows/release.yml) builds the static Linux binary, boots it, and opens a draft GitHub release with `checksums.txt`.
