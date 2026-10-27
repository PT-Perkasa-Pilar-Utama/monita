# Configuration

Monita reads `MONITA_*` environment variables at start. `monita setup` writes them to `/etc/monita/monita.env`, which systemd loads.

| Variable | Means | Production value |
| --- | --- | --- |
| `MONITA_ENV` | `dev`, `test` or `prod`. Required. | `prod` |
| `MONITA_BIND` | Listen address | `127.0.0.1:7466`, behind the reverse proxy |
| `MONITA_BASE_URL` | Public URL, for links and the origin check. Set it to the address people open in the browser: a page at another address, such as the server's IP or another port, cannot sign in. `https://` in prod. Unset, it is `http://localhost:7466`. | `https://monita.example.com` |
| `MONITA_DB_PATH` | Database file | `/var/lib/monita/monita.db` |
| `MONITA_BACKUP_DIR` | Daily backup folder; empty turns backups off | `/var/backups/monita` |
| `MONITA_BACKUP_KEEP` | Backups kept | `7` |
| `MONITA_SMTP_URL` | `smtps://user:pass@host:465`. A fallback: a mail server saved on Admin, Outgoing email wins. With neither, Monita logs emails instead. | your mail server, or leave it empty and set the server on the Outgoing email page |
| `MONITA_MAIL_FROM` | Sender. Unset, it is `Monita <monita@` plus the host of `MONITA_BASE_URL`. | `Monita <monita@example.com>` |
| `MONITA_PING_URL` | A heartbeat Monita pings every 60 s, to be watched from outside | a peer's ping URL |
| `MONITA_TRUST_PROXY` | Read the client IP from `X-Forwarded-For` | `true` behind a proxy |
| `MONITA_DIGEST_UTC_HOUR` | Hour of the daily ticket digest | `1` |
| `MONITA_LOG` | Log filter | `info` |
| `MONITA_LOG_FORMAT` | `pretty` or `json` | `json` |
| `MONITA_TELEGRAM_API` | The Bot API server for the global Telegram bot: Telegram's, or a self-hosted `telegram-bot-api` | `https://api.telegram.org` |

`monita probe` reads two more variables, `MONITA_PROBE_SERVER` and `MONITA_PROBE_TOKEN`. [Locations](locations.md) explains them.

`MONITA_RESET_TOKEN` exists for development only. In prod the reset route is not registered at all.

## Runtime limits

Admins see these values, except secrets, on **System information** under Admin. Changing a variable takes an edit of the env file and `sudo systemctl restart monita`.

Email delivery and the mail server live on **Outgoing email**, with a **Test connection** button and a **Send a test email** form. See [Admin and roles](admin.md#outgoing-email).

A super admin changes these on **Rate limits** without a restart:

| Setting | Default |
| --- | --- |
| Checks at the same time | 50 |
| API calls per minute from one key | 120 |
| Pings per second for one heartbeat | 1 |
| Bug reports per user per day | 5 |
| Bug reports per project per day | 100 |

The time zone for every page is also an admin setting, not a variable. See [Time zone](admin.md#time-zone).

The full reference, with dev and test values, is [spec 11](../technical-specs/11-environment-configuration.md).
