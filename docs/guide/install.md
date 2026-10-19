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
- Monita needs outbound access to the sites it checks, to the mail server, and to each webhook address. Users reach it through a reverse proxy that handles HTTPS.

Measured load: 500 monitors on a 60 s interval used 49 MB of memory and 1 to 8 % of one core. The scheduler kept every check within 1 s of its time. This was a debug build on an Apple M1, and each check called Monita's own `/healthz`. Checks of real sites over HTTPS cost more CPU. The design target for memory is under 100 MB at this load ([NFR-06](../technical-specs/08-non-functional-requirements.md)).

## Get the binary

### From a release

Download the archive for your system from [Releases](https://github.com/PT-Perkasa-Pilar-Utama/monita/releases):

| System | Archive |
| --- | --- |
| Linux, x86 | `monita_linux_amd64.tar.gz` |
| Linux, Arm (Graviton, Ampere, Raspberry Pi 4 and later with a 64-bit OS) | `monita_linux_arm64.tar.gz` |
| macOS, Apple silicon | `monita_darwin_arm64.tar.gz` |
| macOS, Intel | `monita_darwin_amd64.tar.gz` |

The Linux binaries are static. `monita service` needs systemd, so it runs on Linux only. On macOS, run `monita serve` yourself. The macOS binaries are not signed: a copy downloaded in a browser needs `xattr -d com.apple.quarantine monita` before it starts. `curl` downloads do not.

Check the archive against `checksums.txt`, then install it:

```sh
sha256sum -c checksums.txt --ignore-missing
tar -xzf monita_linux_amd64.tar.gz
sudo install -m 0755 monita /usr/local/bin/monita
monita --version
```

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
git checkout v1.0.0                       # or stay on main for unreleased work
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

3. Put the company reverse proxy in front for HTTPS. Setup's defaults, `MONITA_TRUST_PROXY=true` and `MONITA_BIND=127.0.0.1:7466`, assume one. If other hosts can reach the port, they can forge `X-Forwarded-For`. With `MONITA_TRUST_PROXY=false` behind a proxy, every visitor has the proxy's address, so one person can use up the per-address limits, sign-in included, for everyone.

`monita service status`, `monita service logs -f` and `monita service uninstall` cover the rest. `service install` writes the same unit as [deploy/monita.service](../../deploy/monita.service), with this host's paths.

Backups, restore, upgrades and watching Monita itself: [Operations](operations.md).
