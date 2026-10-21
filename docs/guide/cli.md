# Command line

The `monita` binary serves the app and manages a host.

| Command | Does |
| --- | --- |
| `monita serve` | Runs pending migrations, then serves |
| `monita migrate` | Runs pending migrations and exits |
| `monita setup [--yes]` | Writes the env file and creates the first super admin |
| `monita service install` | Creates the `monita` user and folders, writes the systemd unit, starts it, waits for `/healthz` |
| `monita service status` / `logs [-f]` / `uninstall` | Inspect or remove the service |
| `monita whereis [env\|data\|backups\|bin\|service\|logs]` | Prints where things are |
| `monita admin create --email` | Creates a super admin |
| `monita admin reset-password --email` | Sets a new password for a super admin, turns their two-factor sign-in off, and signs them out |
| `monita probe --server <url>` | Runs checks for one location of another Monita and reports them. Reads the token from `MONITA_PROBE_TOKEN`; no env file, no database. See [Locations](locations.md). |
| `monita --version` | Prints the version |

## The env file

Every command reads `/etc/monita/monita.env` when `MONITA_ENV` is not set, so root runs commands without loading the file by hand. `--env-file <path>` names another file. A variable already set in the environment wins over the file.

## Scripted setup

`monita setup --yes` asks nothing. It takes each value from the matching `MONITA_*` variable, then the file, then the default. The first admin comes from `MONITA_ADMIN_EMAIL` and `MONITA_ADMIN_PASSWORD`, which are never written to the file.

```sh
sudo MONITA_BASE_URL=https://monita.example.com MONITA_SMTP_URL=smtps://u:p@mail:465 \
  MONITA_ADMIN_EMAIL=you@example.com MONITA_ADMIN_PASSWORD='a-long-password' \
  monita setup --yes
```

Running it again with the same answers changes nothing. The full contract is [api-specs 10](../api-specs/10-cli.md).
