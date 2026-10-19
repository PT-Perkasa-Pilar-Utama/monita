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
