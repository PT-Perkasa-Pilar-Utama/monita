# 9. System

Policy: [../technical-specs/05-module-definitions.md section 5.8](../technical-specs/05-module-definitions.md#58-platform) and SEC-17.

## 9.1 `GET /healthz`

| Part | Contract |
| --- | --- |
| Purpose | Tell a peer Monita, a load balancer or an operator whether Monita works. |
| Access | public, no rate limit |
| Behavior | Run `SELECT 1`. Read the scheduler's last tick. Count outbox rows that are due and unsent. |
| Errors | None beyond the 503 below |

| Condition | Status | Body |
| --- | --- | --- |
| DB works, scheduler ticked in the last 30 s | 200 | `{"status":"ok","db":"ok","scheduler_lag_s":1,"outbox_pending":0,"version":"1.0.0"}` |
| DB fails | 503 | `{"status":"fail","db":"fail","scheduler_lag_s":1,"outbox_pending":null,"version":"1.0.0"}` |
| Scheduler stalled | 503 | `{"status":"fail","db":"ok","scheduler_lag_s":45,"outbox_pending":0,"version":"1.0.0"}` |

Not wrapped in the `data` envelope, so probes can read `status` directly. (AC-34.01)

## 9.2 `POST /admin/reset-state`

| Part | Contract |
| --- | --- |
| Purpose | Put the database back into a known state for QA and e2e. |
| Mounted | Only when `MONITA_ENV` is `dev` or `test`. In `prod` the route is not registered and returns 404. |
| Access | `dev`: super_admin session. `test`: header `X-Reset-Token` equal to `MONITA_RESET_TOKEN`. |
| Input | `{"seed": "dev" \| "qa"}` |
| Behavior | Pause the scheduler. Delete all rows. Run migrations. Load `seeds/<seed>.sql`. Resume. Synchronous. |
| Output | 204 |
| Errors | 400 `invalid_request` for another seed. 401 `unauthorized` for a bad token or no session. |
