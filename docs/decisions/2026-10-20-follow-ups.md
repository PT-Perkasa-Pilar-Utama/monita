# Follow-ups closed: decisions

Date: 2026-10-20

`docs/followups.md` held work deferred during the fast frontend cycles. Three of its rows asked for a decision, not only a fix. The user delegated those to the advisor. This log records them; the file is deleted once every row is done.

| # | Question | Decision | Why |
|---|----------|----------|-----|
| Q21 | Revoking a ref ID that is already revoked answers 204. Should it be 404? | Keep 204. A second revoke changes nothing and keeps the first `revoked_at`. | Revoke is idempotent, like an HTTP `DELETE`. A tech lead who clicks twice, or two who click at once, should not see an error for an outcome they wanted. |
| Q22 | The public board answers 200 with an empty list for `?stage=triage`, and 404 for any other stage it does not show. Which is right? | 404 for Triage too. The public board has three columns: backlog, in progress and done. | One rule for every column the public board does not have. An empty 200 also suggests that Triage is a public column that happens to be empty (SEC-14 keeps it private). |
| Q23 | A supervisor sees a monitor URL in full, query included (`?token=...`). Mask it? | Yes. For a supervisor, everything from the first `?` or `#` reads `(hidden)`: `https://api.x.co/health?(hidden)`. The monitor page says once why values read `(hidden)`. | Names and passwords in a URL are already refused, but a token in the query is common (`?apikey=`). SEC-22 hides header values and the body for the same reason; the query is the same kind of value. |
