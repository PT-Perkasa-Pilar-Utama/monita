# Seeds

Loaded only by `POST /admin/reset-state`, which exists in `dev` and `test` and never in `prod`.

Load one into a dev server: start it with a reset token, then reset once.

```sh
MONITA_RESET_TOKEN=dev make run
curl -s -X POST http://127.0.0.1:8080/admin/reset-state -H 'x-reset-token: dev' \
  -H 'content-type: application/json' -d '{"seed":"dev"}'
```

| Account | Role | Password |
| --- | --- | --- |
| `admin@monita.test` | super admin | `monita-dev-password` |
| `tl@monita.test` | tech lead | `monita-dev-password` |
| `rina@monita.test` | tech lead, `demo.sql` only | `monita-dev-password` |

`dev.sql` also creates the project `demo`, owned by the tech lead. `qa.sql` creates no project: each e2e test creates its own.

`demo.sql` holds three projects with 90 days of history, open and resolved incidents, and tickets in every stage. It feeds the static demo: `cd e2e && bun demo/snapshot.ts` builds it into `target/site`, and `--screens ../docs/assets/screens` also refreshes the README screenshots. CI publishes it to GitHub Pages from `main`.
