# 10. Integration Points

| System | Direction | Contract | Failure mode | Fallback |
| --- | --- | --- | --- | --- |
| Monitored URLs | Out | HTTP(S) GET, monitor timeout, max 5 redirects (none when the expected status names a 3xx), 1 MB body | Timeout, DNS, TLS, status mismatch | Recorded as a failed check. Feeds the incident engine. |
| SMTP relay | Out | SMTP with STARTTLS or TLS, `MONITA_SMTP_URL` | Slow or down | Outbox retries at 1, 5, 15 min. Then `failed_at` and an error log. Checks never wait on SMTP. |
| Webhook URLs | Out | POST JSON with `text`, 10 s timeout | Non-2xx or timeout | Same outbox retries. |
| Peer Monita or Better Stack | Out | GET `MONITA_PING_URL` every 60 s, 10 s timeout | Down | Warning log only. The peer raises the alert from its side. |
| Project backends | In | REST JSON with an API key. See [api-specs](../api-specs/). | Bad key, over the limit | 401, 403, 429 with `Retry-After`. |
| Project jobs | In | `GET`, `HEAD` or `POST /ping/{token}` and `/ping/{token}/fail` | Over 1/s | 429. A refused ping is not stored; the ping accepted in that second keeps the heartbeat alive. |
| Custom status pages | In | `GET /api/v1/public/status/{slug}`, CORS `*` | Over the limit | 429 |

## 10.1 Caching

- The public status JSON and page are built on each request. At NFR-05 this needs no cache.
- `Cache-Control: no-store` on admin pages. `Cache-Control: max-age=30` on public status pages and JSON.
- Static assets: `Cache-Control: max-age=31536000, immutable`. The file name holds a content hash computed at build time.
