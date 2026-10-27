# 10. Integration Points

| System | Direction | Contract | Failure mode | Fallback |
| --- | --- | --- | --- | --- |
| Monitored URLs | Out | HTTP(S) `GET`, `HEAD`, `POST` or `PUT` with the monitor's headers, body and optional sign-in from a sealed secret, monitor timeout, max 5 redirects (none when the expected status names a 3xx), 1 MB body | Timeout, DNS, TLS, status mismatch | Recorded as a failed check. Feeds the incident engine. |
| Monitored hosts (TCP, DNS, ping) | Out | TCP connect to `host:port`. A lookup with the server's resolver. One ICMP echo over an unprivileged datagram socket, or the system `ping` where the socket is refused and on Windows (v1.1 Q16, Q24, Q40) | Timeout, refused, no answer | Recorded as a failed check. Feeds the incident engine. |
| SMTP relay | Out | SMTP with STARTTLS or TLS, from the mail server saved on the admin Email page, else `MONITA_SMTP_URL` | Slow or down | Outbox retries at 1, 5, 15 min. Then `failed_at` and an error log. Checks never wait on SMTP. |
| Webhook URLs | Out | POST JSON with `text` and `event` ([05 section 5.5](05-module-definitions.md#55-notify)), 10 s timeout, no redirect | Non-2xx or timeout | Same outbox retries. |
| Telegram Bot API | Out | The global bot (v1.1 Q33): `getMe` and `getWebhookInfo` when a super admin saves the token, `getUpdates` when a card checks its connect link, `sendMessage` for each alert. Base URL `MONITA_TELEGRAM_API` | Bad token, a webhook set on the bot (409), timeout | The page names the cause. A failed alert uses the outbox retries. |
| PagerDuty Events API | Out | POST to `https://events.pagerduty.com/v2/enqueue` with the card's routing key. A certificate warning triggers and never resolves | Non-2xx or timeout | Same outbox retries. |
| ntfy | Out | POST to the card's ntfy server and topic | Non-2xx or timeout | Same outbox retries. |
| Peer Monita or Better Stack | Out | GET `MONITA_PING_URL` every 60 s, 10 s timeout | Down | Warning log only. The peer raises the alert from its side. |
| Project backends | In | REST JSON with an API key. See [api-specs](../api-specs/). | Bad key, over the limit | 401, 403, 429 with `Retry-After`. |
| Project jobs | In | `GET`, `HEAD` or `POST /ping/{token}` and `/ping/{token}/fail` | Over 1/s | 429. A refused ping is not stored; the ping accepted in that second keeps the heartbeat alive. |
| Custom status pages | In | `GET /api/v1/public/status/{slug}`, CORS `*` | None. No limit applies (v1.1 Q45). | None |
| Location probes | In | `GET /api/v1/probe/work` and `POST /api/v1/probe/results` with an `ml_` token ([14](14-locations.md)) | Bad token, over 300 calls a minute | 401, 429 with `Retry-After`. The probe waits and asks again. |

## 10.1 Caching

- The public status JSON and page are built on each request. At NFR-05 this needs no cache.
- `Cache-Control: no-store` on every signed-in `/api/ui/` response. `Cache-Control: public, max-age=30` on the public status and ticket-board JSON.
- Static assets: `Cache-Control: max-age=31536000, immutable`. The file name holds a content hash computed at build time.
