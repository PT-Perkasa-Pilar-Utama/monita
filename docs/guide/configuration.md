# Configuration

Monita reads `MONITA_*` environment variables at start. `monita setup` writes them to `/etc/monita/monita.env`, which systemd loads.

| Variable | Means | Production value |
| --- | --- | --- |
| `MONITA_ENV` | `dev`, `test` or `prod`. Required. | `prod` |
| `MONITA_BIND` | Listen address | `127.0.0.1:7466`, behind the reverse proxy |
| `MONITA_BASE_URL` | Public URL, for links and the origin check. `https://` in prod. | `https://monita.example.com` |
| `MONITA_DB_PATH` | Database file | `/var/lib/monita/monita.db` |
| `MONITA_BACKUP_DIR` | Daily backup folder; empty turns backups off | `/var/backups/monita` |
| `MONITA_BACKUP_KEEP` | Backups kept | `7` |
| `MONITA_SMTP_URL` | `smtps://user:pass@host:465`. A mail server saved on Admin, Email replaces it; with neither, Monita logs emails instead | your mail server, or set it on the Email page |
| `MONITA_MAIL_FROM` | Sender | `Monita <monita@example.com>` |
| `MONITA_PING_URL` | A heartbeat Monita pings every 60 s, to be watched from outside | a peer's ping URL |
| `MONITA_TRUST_PROXY` | Read the client IP from `X-Forwarded-For` | `true` behind a proxy |
| `MONITA_DIGEST_UTC_HOUR` | Hour of the daily ticket digest | `1` |
| `MONITA_LOG` | Log filter | `info` |
| `MONITA_LOG_FORMAT` | `pretty` or `json` | `json` |

`MONITA_RESET_TOKEN` exists for development only. In prod the reset route is not registered at all.

## Runtime limits

Admins see every value on this page, except secrets, on the **System** page under Admin. That page also shows whether email delivery works and sends a test email. Changing a variable takes an edit of the env file and `sudo systemctl restart monita`.

A super admin changes these on **System** without a restart:

| Setting | Default |
| --- | --- |
| Checks running at once | 50 |
| Public requests per minute per IP | 60 |
| API requests per minute per key | 120 |
| Pings per second per heartbeat | 1 |
| Tickets per ref ID per day | 5 |
| Tickets per project per day | 100 |

The full reference, with dev and test values, is [spec 11](../technical-specs/11-environment-configuration.md).
