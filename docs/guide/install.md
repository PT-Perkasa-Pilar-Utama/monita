# Install

Monita runs as one systemd service on a Linux VM. These steps assume Ubuntu or Debian and root access. To try it on a laptop first, see [Getting started](getting-started.md).

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

   [Configuration](configuration.md) describes every variable. For a scripted install, `monita setup --yes` takes the answers from `MONITA_*` variables ([Command line](cli.md)).

3. Install and start the service. It creates the `monita` user and the folders, writes the systemd unit, and waits for `/healthz`:

   ```sh
   sudo monita service install
   monita whereis               # env file, database, backups, unit, logs
   ```

4. Put the company reverse proxy in front for HTTPS. Setup's defaults, `MONITA_TRUST_PROXY=true` and `MONITA_BIND=127.0.0.1:8080`, assume one. If other hosts can reach the port, they can forge `X-Forwarded-For`.

`monita service status`, `monita service logs -f` and `monita service uninstall` cover the rest. `service install` writes the same unit as [deploy/monita.service](../../deploy/monita.service), with this host's paths.

Backups, restore, upgrades and watching Monita itself: [Operations](operations.md).
