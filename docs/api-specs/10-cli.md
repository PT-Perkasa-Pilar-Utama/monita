# 10. Command line

The `monita` binary. `admin create` is in [02-authentication.md section 2.1](02-authentication.md#21-cli-monita-admin-create). Variables: [../technical-specs/11-environment-configuration.md](../technical-specs/11-environment-configuration.md).

| Command | Does |
| --- | --- |
| `monita serve` | Runs pending migrations, then serves HTTP and the background loops. |
| `monita migrate` | Runs pending migrations and exits. |
| `monita setup [--yes]` | Writes the env file and creates the first super admin (10.2). |
| `monita service install\|uninstall\|status\|logs [-f]` | Manages the systemd unit (10.3). |
| `monita whereis [env\|data\|backups\|bin\|service\|logs]` | Prints where things are (10.4). |
| `monita admin create --email` | Creates a super admin (2.1). |
| `monita admin reset-password --email` | Sets a new password for a super admin, turns their two-factor sign-in off, and signs them out everywhere. |
| `monita admin reset-sign-in --email` | For any active user who lost the authenticator and the recovery codes: turns two-factor off, stops the old password, and queues an invite email (10.6). |
| `monita --version` | Prints the version. |

Every error prints `Error: <message>` to stderr and exits 1. A usage error exits 2.

## 10.1 The env file

| Part | Contract |
| --- | --- |
| Flag | `--env-file <path>` on any command. |
| Default | Without the flag, Monita reads `/etc/monita/monita.env` when it exists and `MONITA_ENV` is unset. systemd sets `MONITA_ENV` from the same file, so `serve` under the unit and `make run` never read it twice. |
| Order | A variable set in the process environment wins over the file. |
| Format | `KEY=value` lines. `#` starts a comment. A value in single quotes is literal; a value in double quotes takes `\"` and `\\`. systemd's `EnvironmentFile=` and a POSIX shell read the file the same way. |
| Errors | The file does not exist: exit 1, `<path> does not exist`. It exists but this user may not read it: exit 1, `could not read <path>; it is readable by root only`. A bad value fails before anything starts, and the message names the variable. |

## 10.2 `monita setup`

| Part | Contract |
| --- | --- |
| Input | It asks, in order: `MONITA_BASE_URL`, `MONITA_BIND`, `MONITA_TRUST_PROXY`, `MONITA_DB_PATH`, `MONITA_BACKUP_DIR`, `MONITA_SMTP_URL`, `MONITA_MAIL_FROM`. Each prompt shows the value in the file, or the prod value of spec 11. Enter keeps it; `-` leaves it empty. Then it asks the first super admin's email (blank skips) and the password twice, without echo. |
| `--yes` | Asks nothing. Each value comes from the same `MONITA_*` variable, then the file, then the default. The admin comes from `MONITA_ADMIN_EMAIL` and `MONITA_ADMIN_PASSWORD`; without the email, no admin is created. The email without the password is refused before the file is written. |
| Behavior | 1. Sets `MONITA_ENV=prod`. 2. Validates the result as `serve` would. 3. Writes the file through a temporary file and a rename, mode 0600. A key already in the file keeps its line; other lines and comments stay; a new key goes at the end. 4. Creates the admin. An email that already has an account is kept. |
| Output | `wrote <path>` or `nothing changed in <path>`, then `created super admin <email>` or `an account with <email> already exists; kept`, then `next: sudo monita service install`. A second run with the same answers changes nothing and exits 0. |
| Errors | An invalid value: exit 1, the file is not written, and the message names the variable. A value with a single quote or a line break: exit 1. Two different passwords: exit 1, no account. |
| Not stored | The admin password. |

## 10.3 `monita service`

Linux with systemd, as root. On another system each subcommand exits 1 and names `monita serve`.

| Subcommand | Contract |
| --- | --- |
| `install` | Needs an env file (10.1) and a binary outside `/home` and `/root`, which the unit hides. 1. Creates the system user `monita` if missing. 2. Creates the data folder and the backup folder, mode 0700, and gives them and their files to `monita`. 3. Writes `/etc/systemd/system/monita.service` from `deploy/monita.service`, with `ExecStart`, `EnvironmentFile` and `ReadWritePaths` set to this binary, this env file and these folders. 4. Runs `systemctl daemon-reload`, `enable` and `restart`. 5. Waits up to 20 s for `/healthz` on `MONITA_BIND`. Exit 1 if it does not answer. Running it again after an upgrade restarts the service on the new binary. |
| `uninstall` | `systemctl disable --now monita`, removes the unit, reloads. The env file, the data, the backups and the user stay. |
| `status` | `systemctl status monita --no-pager`, with its exit code. |
| `logs [-f]` | The last 200 lines of `journalctl -u monita`; `-f` follows. |

Paths in the unit must be absolute and free of spaces. Root runs `setup` and the `admin` commands, so after each of them Monita gives the database files the owner of their folder.

## 10.4 `monita whereis`

Without an argument it prints one `label value` row for `env`, `data`, `backups`, `bin`, `service` and `logs`. With one, it prints that value alone, for scripts: `monita whereis data` prints the database path. It prints the values as written and checks none of them, so it works when the settings are what is wrong. `service uninstall`, `status` and `logs` read no settings either.

## 10.5 `monita probe`

Runs checks for one location of another Monita (technical-specs 14). It reads no env file and opens no database.

| Option | Variable | Meaning |
| --- | --- | --- |
| `--server <url>` | `MONITA_PROBE_SERVER` | The Monita to report to, such as `https://monita.company.co.id` |
| `--token <token>` | `MONITA_PROBE_TOKEN` | The location's token. Prefer the variable: a command line shows in `ps`. |
| `--concurrency <n>` | | Checks at once, 1 to 50, default 20 |

With a free slot, it asks for as many jobs as it has free slots with `wait=25` (a long poll, 4.5): the server answers when a check falls due, or after 25 s with none. An idle probe so calls about twice a minute. A server that answers at once, from before long polling, is asked again a second later. The probe posts results from its own task as checks finish, in batches of up to 50. A server it cannot reach is logged and tried again 5 seconds later. A 401 means the token was deleted or replaced: it logs that and exits with status 1. It stops on Ctrl-C or SIGTERM after the checks in flight.

### `monita probe install` and `monita probe uninstall`

Linux with systemd, as root. Other systems get exit 1 with a pointer to Docker.

| Part | Contract |
| --- | --- |
| Input | `--server <url>` or `MONITA_PROBE_SERVER`; the token from `MONITA_PROBE_TOKEN` or `--token`. |
| Checks | The server must start with `http://` or `https://`, and the token with `ml_`. The binary must not sit under `/home` or `/root`, which the unit hides. Nothing is written when a check fails. |
| Writes | `/etc/monita/probe.env` with mode 0600 (`MONITA_PROBE_SERVER`, `MONITA_PROBE_TOKEN`), and `/etc/systemd/system/monita-probe.service` from `deploy/monita-probe.service` with this binary in `ExecStart`. |
| Then | `systemctl daemon-reload`, `enable` and `restart monita-probe`. |
| Uninstall | `systemctl disable --now monita-probe`, then removes the unit and the env file. |

## 10.6 `monita admin reset-sign-in`

The fail-safe for a user who lost the authenticator and the recovery codes. It works for any active user: super admin, tech lead or supervisor. It runs offline like the other `admin` commands, so the server must be stopped.

| Part | Contract |
| --- | --- |
| Input | `--email <address>`. |
| Behavior | Does what a super admin's new invite does (2.3): deletes the user's sessions and older invites, turns two-factor off, and makes the old password stop working. Then it queues an invite email and prints its link. |
| Output | `Two-factor is off for <email> and the old password no longer works.` Then the link. The email goes out when Monita runs. The link works once and lasts 72 hours. |
| Errors | An unknown or disabled user: exit 1, `No active user has this email.` |

`monita admin reset-password` stays the way to set a super admin's password directly.
