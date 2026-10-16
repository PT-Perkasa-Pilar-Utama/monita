# 11. Environment Configuration

Monita reads its configuration from environment variables at start. systemd loads them from `/etc/monita/monita.env`. Runtime limits live in the `settings` table, not here.

| Variable | Meaning | dev | test (e2e) | prod |
| --- | --- | --- | --- | --- |
| `MONITA_ENV` | `dev`, `test` or `prod`. Gates reset-state. Required: Monita refuses to start without it. | `dev` | `test` | `prod` |
| `MONITA_BIND` | Listen address | `127.0.0.1:8080` | `127.0.0.1:18080` | `127.0.0.1:8080` behind the reverse proxy. With `MONITA_TRUST_PROXY=true`, a port others can reach lets them forge `X-Forwarded-For`. |
| `MONITA_BASE_URL` | Public URL, for links and the Origin check | `http://localhost:8080` | `http://127.0.0.1:18080` | `https://monita.company.co.id` |
| `MONITA_DB_PATH` | Database file | `./data/monita.db` | `./data/e2e.db` | `/var/lib/monita/monita.db` |
| `MONITA_BACKUP_DIR` | Backup folder. Empty disables backup. | empty | empty | `/var/backups/monita` |
| `MONITA_BACKUP_KEEP` | Backups kept | 7 | 7 | 7 |
| `MONITA_SMTP_URL` | `smtp://` or `smtps://user:pass@host:port`. Empty logs emails instead of sending. | empty | empty | set |
| `MONITA_MAIL_FROM` | Sender | `monita@localhost` | `monita@localhost` | `Monita <monita@company.co.id>` |
| `MONITA_PING_URL` | Peer or outside heartbeat. Empty disables. | empty | empty | peer URL |
| `MONITA_TRUST_PROXY` | Read the client IP from `X-Forwarded-For` | `false` | `false` | `true` behind the reverse proxy |
| `MONITA_DIGEST_UTC_HOUR` | Hour of the ticket digest | `1` | `1` | `1` (08:00 WIB) |
| `MONITA_RESET_TOKEN` | Header token for reset-state in `dev` and `test` | optional; unset disables reset-state | random per run | absent |
| `MONITA_LOG` | Log filter | `debug` | `info` | `info` |
| `MONITA_LOG_FORMAT` | `pretty` or `json` | `pretty` | `pretty` | `json` |

Reset-state exists only where `MONITA_ENV` is `dev` or `test`. In prod, `MONITA_RESET_TOKEN` is absent and the route is never registered.

Monita refuses to start when `MONITA_ENV=prod` and `MONITA_BASE_URL` is not `https://`.
