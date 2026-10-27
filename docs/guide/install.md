# Install

Monita runs as one systemd service on a Linux VM. These steps assume Ubuntu or Debian and root access. To try it on a laptop first, see [Getting started](getting-started.md).

## Requirements

| | Minimum | Recommended for 500 monitors |
| --- | --- | --- |
| CPU | 1 vCPU | 2 vCPU |
| Memory | 512 MB | 1 GB |
| Disk | 5 GB | 20 GB |
| System | 64-bit Linux with systemd, x86 or Arm | Same |

- The release binary is static. It needs no other libraries, no database server and no runtime.
- The disk holds the database and 7 backups. Each backup is a full copy of the database, so plan for 8 times its size.
- The database keeps 7 days of check results. At 500 monitors on a 60 s interval, that is about 5 million rows. The design target is under 2 GB ([NFR-09](../technical-specs/08-non-functional-requirements.md)).
- A ping check uses an ICMP socket first. On Linux that needs the sysctl `net.ipv4.ping_group_range` to cover Monita's group. Most systemd distributions and Docker set `0 2147483647` already. Check with `sysctl net.ipv4.ping_group_range`. Where the socket is refused, and on Windows, Monita runs the system `ping` command. The Docker image has no `ping` command, but Docker sets the sysctl, so it works.
- Monita needs outbound access to the sites it checks, to the mail server, and to each webhook address. Users reach it through a reverse proxy that handles HTTPS.

Measured load (2026-10-20): the 1.0.1 release image ran in Docker Desktop on an Apple M1 (arm64, a VM with 2 vCPU) and checked 500 URLs on a 60 s interval against a local nginx. It used 17.6 MiB of memory idle, and 21.2 MiB (median) to 24.2 MiB (peak) under load, with 0.8 % of one core on average. The nginx received 498 requests a minute, so every check ran. After 10 minutes the data folder held 2.7 MB. On 2026-10-27, the same test ran in colima on the same Mac with 1.0.1 and a 1.1.0 build side by side. Under load, `docker stats` counted 20.4 MiB (median) for 1.0.1 and 3.6 MiB for 1.1.0, both at about 1.2 % of one core, and both did every check. Checks of real sites over HTTPS cost more CPU and memory. The design target for memory is under 100 MB at this load ([NFR-06](../technical-specs/08-non-functional-requirements.md)).

## Get the binary

### From a release

Download the archive for your system from [Releases](https://github.com/PT-Perkasa-Pilar-Utama/monita/releases):

| System | Archive |
| --- | --- |
| Linux, x86 | `monita_linux_amd64.tar.gz` |
| Linux, Arm (Graviton, Ampere, Raspberry Pi 4 and later with a 64-bit OS) | `monita_linux_arm64.tar.gz` |
| macOS, Apple silicon | `monita_darwin_arm64.tar.gz` |
| macOS, Intel | `monita_darwin_amd64.tar.gz` |
| Windows, x86 | `monita_windows_amd64.zip` |
| Windows, Arm | `monita_windows_arm64.zip` |
| Docker, any system | `ghcr.io/pt-perkasa-pilar-utama/monita:1`, see [With Docker](#with-docker) |

The Linux binaries are static. `monita service` needs systemd, so it runs on Linux only. On macOS and Windows, run `monita serve` yourself. On Windows, keep the settings in a file and name it: `monita.exe serve --env-file monita.env`. Start from `monita.env.example` in the archive, and set `MONITA_DB_PATH` to a folder of your own, such as `C:\monita\monita.db`. Windows does not use Unix file modes, so protect that folder with its own permissions. The Windows binaries are not signed either, so SmartScreen may ask before the first start. The macOS binaries are not signed: a copy downloaded in a browser needs `xattr -d com.apple.quarantine monita` before it starts. `curl` downloads do not.

Check the archive against `checksums.txt`, then install it:

```sh
sha256sum -c checksums.txt --ignore-missing
tar -xzf monita_linux_amd64.tar.gz
sudo install -m 0755 monita /usr/local/bin/monita
monita --version
```

### With Docker

The image `ghcr.io/pt-perkasa-pilar-utama/monita` runs on Linux, and on Windows and macOS through Docker Desktop. It holds the same static binary as the Linux archives, for `linux/amd64` and `linux/arm64`. It has no shell and runs as a user without root rights. Tags: `1.1.0` for one release, `1.1` and `1` for the newest in that line, and `latest`.

Data lives in the volume at `/data`. The commands below are one line each, so they work in PowerShell, Command Prompt and a Unix shell.

To try it on a laptop:

1. Create the volume:

   ```sh
   docker volume create monita-data
   ```

2. Create the first super admin. The command asks for a password. Run it before the server starts: the database opens for one program at a time.

   ```sh
   docker run --rm -it -v monita-data:/data -e MONITA_ENV=dev ghcr.io/pt-perkasa-pilar-utama/monita:1 admin create --email you@company.co.id
   ```

3. Start Monita:

   ```sh
   docker run -d --name monita --restart unless-stopped -p 127.0.0.1:7466:7466 -v monita-data:/data -e MONITA_ENV=dev ghcr.io/pt-perkasa-pilar-utama/monita:1
   ```

4. Open `http://localhost:7466` and sign in. `127.0.0.1` works too. The `-p 127.0.0.1:7466:7466` publish makes Monita reachable from this computer only. Sign-in checks that the page and `MONITA_BASE_URL` (default `http://localhost:7466`) have the same address. If you publish another port or open Monita from another machine, set `MONITA_BASE_URL` to that address.

`MONITA_ENV=dev` allows an `http://` address and logs more. On a server, keep the image's `MONITA_ENV=prod` and put Monita behind a web server that handles HTTPS. A Compose file for a server:

```yaml
services:
  monita:
    image: ghcr.io/pt-perkasa-pilar-utama/monita:1
    restart: unless-stopped
    ports: ["127.0.0.1:7466:7466"]
    volumes: ["monita-data:/data"]
    environment:
      MONITA_BASE_URL: https://monita.company.co.id
      MONITA_SMTP_URL: smtps://user:password@mail.company.co.id:465
      MONITA_TRUST_PROXY: "true"
volumes:
  monita-data:
```

[deploy/nginx/compose.yaml](../../deploy/nginx/compose.yaml) runs the same with nginx in front for HTTPS. Only nginx publishes ports there, so no one outside can forge `X-Forwarded-For`.

Create the first super admin with `docker compose run --rm monita admin create --email you@company.co.id` before the first `docker compose up -d`. It asks for the password, at least 8 characters. If Monita is already running, run `docker compose stop` first: the running server holds the database, and `admin create` fails with "is open in another Monita process". Then start it again with `docker compose up -d`. Each variable is in [Configuration](configuration.md). `monita service` does not apply in a container: Docker restarts it.

#### Upgrade from v1.0

The Compose file above pins `ghcr.io/pt-perkasa-pilar-utama/monita:1`. The release job moves the `1` tag to the newest 1.x release. So this upgrades a v1.0 install:

```sh
docker compose pull && docker compose up -d
```

Monita migrates the database when it starts (migrations 0024 to 0049). Back up the data volume first. The image does not turn on Monita's daily backups: set `MONITA_BACKUP_DIR` if you want them. The first start also switches the file to give deleted space back to the disk. That rewrite needs free disk space equal to the size of the file.

The upgrade was tested on 2026-10-27, from the `1.0.1` image to a 1.1.0 image built from the release commit. v1.0.0 and v1.0.1 have the same database, so the test covers both. The v1.0.1 setup ran for 6 minutes with real checks:

- two projects, three HTTP monitors (one failing), and two heartbeats (one pinged every 20 s, one never pinged)
- a critical alert route with two emails and a webhook, and ticket alerts with a webhook
- an API key, a ticket filed with a ref ID, and an open manual incident

Then the v1.1 image started on the same volume. Every check passed:

- Each migration ran once. `integrity_check` returned ok and `foreign_key_check` found nothing.
- The admin password still signed in. Every project page and admin page answered 200.
- Each monitor kept its v1.0 checks and went on checking. Migration 0046 built the 15-minute chart totals from the old checks.
- The heartbeat's old ping URL still answered 200, and its pings were stored.
- The three open incidents stayed open, once each. No incident opened for a healthy monitor.
- The routes became alert cards with the same emails and webhooks, and a test alert went out.
- The API key still issued ref IDs. It became a `ref_ids` key, so give it a new scope before it uses the project key API.
- The failing monitor, fixed through the new form, closed its incident on the next passing check.
- `sqlite3` read the live file from another container, and Monita kept working.

To go back to v1.0.1, restore the backup you took before the upgrade. v1.0.1 does not know the tables that v1.1 added.

### From source

Build on a workstation or in CI, then copy the binary to the server. The release build links the whole program at once (fat LTO). It takes about 6 minutes on a laptop.

You need:

| Tool | Version | Install |
| --- | --- | --- |
| Rust | 1.97.1 | [rustup](https://rustup.rs). `rust-toolchain.toml` picks the version on the first `cargo` command. |
| Bun | 1.4 | [bun.sh](https://bun.sh). It builds the web pages. |
| C compiler | Any | `sudo apt-get install -y build-essential` on Debian or Ubuntu. Xcode Command Line Tools on macOS. |
| git, make | Any | Your system's package manager |

To build the static Linux binary that a release ships, run these commands on 64-bit x86 Linux:

```sh
git clone https://github.com/PT-Perkasa-Pilar-Utama/monita.git
cd monita
git checkout v1.1.0                       # or stay on main for unreleased work
sudo apt-get install -y musl-tools
rustup target add x86_64-unknown-linux-musl
make release
```

The binary is `target/x86_64-unknown-linux-musl/release/monita`. Copy it to the server and install it:

```sh
sudo install -m 0755 monita /usr/local/bin/monita
monita --version
```

To build for the machine you are on, for example to try it on macOS, run `make web` and then `cargo build --release`. The binary is `target/release/monita`. `monita service` works on Linux only.

The web pages are built into the binary. A binary built without `make web` (or `make release`, which runs it) answers 503 on every page.

## Set it up

1. Answer the setup questions. Setup writes `/etc/monita/monita.env` (mode 0600) and creates the first super admin:

   ```sh
   sudo monita setup
   ```

   [Configuration](configuration.md) describes every variable. For a scripted install, `monita setup --yes` takes the answers from `MONITA_*` variables ([Command line](cli.md)).

2. Install and start the service. It creates the `monita` user and the folders, writes the systemd unit, and waits for `/healthz`:

   ```sh
   sudo monita service install
   monita whereis               # env file, database, backups, unit, logs
   ```

3. Put a reverse proxy in front for HTTPS. [deploy/nginx/monita.conf](../../deploy/nginx/monita.conf) is a ready nginx config: copy it to `/etc/nginx/conf.d/`, set your host name and certificate paths, and reload nginx. Pages also load faster behind it: over HTTPS, browsers take the smaller brotli copy of the web bundle. Setup's defaults, `MONITA_TRUST_PROXY=true` and `MONITA_BIND=127.0.0.1:7466`, assume a proxy. If other hosts can reach the port, they can forge `X-Forwarded-For`. With `MONITA_TRUST_PROXY=false` behind a proxy, every visitor has the proxy's address, so a project's IP allowlist sees only the proxy.

`monita service status`, `monita service logs -f` and `monita service uninstall` cover the rest. `service install` writes the same unit as [deploy/monita.service](../../deploy/monita.service), with this host's paths.

Backups, restore, upgrades and watching Monita itself: [Operations](operations.md).
