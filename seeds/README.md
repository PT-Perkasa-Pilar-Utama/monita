# Seeds

Loaded only by `POST /admin/reset-state`, which exists in `dev` and `test` and never in `prod`.

| Account | Role | Password |
| --- | --- | --- |
| `admin@monita.test` | super admin | `monita-dev-password` |
| `tl@monita.test` | tech lead | `monita-dev-password` |

`dev.sql` also creates the project `demo`, owned by the tech lead. `qa.sql` creates no project: each e2e test creates its own.

`demo.sql` holds three projects with 90 days of history, open and resolved incidents, and tickets in every stage. It feeds the static demo: `cd e2e && bun demo/snapshot.ts` builds it into `target/site`, and `--screens ../docs/assets/screens` also refreshes the README screenshots. CI publishes it to GitHub Pages from `main`.
