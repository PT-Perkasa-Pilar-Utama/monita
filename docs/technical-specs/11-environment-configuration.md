# 11. Environment Configuration

Monita reads its configuration from environment variables at start. systemd loads them from `/etc/monita/monita.env`, which `monita setup` writes. A command run by hand reads the same file when `MONITA_ENV` is unset ([api-specs 10.1](../api-specs/10-cli.md#101-the-env-file)). Runtime limits, the time zone offset and the saved mail server live in the `settings` table, not here ([06 section 6.3.14](06-data-model.md)).

| Variable | Meaning | dev | test (e2e) | prod |
| --- | --- | --- | --- | --- |
| `MONITA_ENV` | `dev`, `test` or `prod`. Gates reset-state. Required: Monita refuses to start without it. | `dev` | `test` | `prod` |
| `MONITA_BIND` | Listen address | `127.0.0.1:7466` | `127.0.0.1:18080` | `127.0.0.1:7466` behind the reverse proxy. With `MONITA_TRUST_PROXY=true`, a port others can reach lets them forge `X-Forwarded-For`. |
| `MONITA_BASE_URL` | Public URL, for links and the Origin check | `http://localhost:7466` | `http://127.0.0.1:18080` | `https://monita.company.co.id` |
| `MONITA_DB_PATH` | Database file | `./data/monita.db` | `./data/e2e.db` | `/var/lib/monita/monita.db` |
| `MONITA_BACKUP_DIR` | Backup folder. Empty disables backup. | empty | empty | `/var/backups/monita` |
| `MONITA_BACKUP_KEEP` | Backups kept | 7 | 7 | 7 |
| `MONITA_SMTP_URL` | Fallback mail server: `smtp://` or `smtps://user:pass@host:port`. Used only while no super admin has saved a mail server on the admin Email page. With neither, Monita logs emails instead of sending. | empty | empty | set, or empty when the Email page holds the server |
| `MONITA_MAIL_FROM` | Sender for the fallback server. A saved mail server has its own sender. | `monita@localhost` | `monita@localhost` | `Monita <monita@company.co.id>` |
| `MONITA_PING_URL` | Peer or outside heartbeat. Empty disables. | empty | empty | peer URL |
| `MONITA_TRUST_PROXY` | Read the client IP from `X-Forwarded-For` | `false` | `false` | `true` behind the reverse proxy |
| `MONITA_DIGEST_UTC_HOUR` | Hour of the ticket digest | `1` | `1` | `1` (08:00 WIB) |
| `MONITA_RESET_TOKEN` | Header token for reset-state in `dev` and `test` | optional; unset disables reset-state | random per run | absent |
| `MONITA_LOG` | Log filter | `info,monita=debug` | `info` | `info` |
| `MONITA_LOG_FORMAT` | `pretty` or `json` | `pretty` | `pretty` | `json` |

Reset-state exists only where `MONITA_ENV` is `dev` or `test`. In prod, `MONITA_RESET_TOKEN` is absent and the route is never registered.

Monita refuses to start when `MONITA_ENV=prod` and `MONITA_BASE_URL` is not `https://`.

`monita setup --yes` also reads `MONITA_ADMIN_EMAIL` and `MONITA_ADMIN_PASSWORD` for the first super admin. It never writes them to the file, and `serve` ignores them.
