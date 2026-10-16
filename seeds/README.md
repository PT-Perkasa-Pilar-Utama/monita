# Seeds

Loaded only by `POST /admin/reset-state`, which exists in `dev` and `test` and never in `prod`.

| Account | Role | Password |
| --- | --- | --- |
| `admin@monita.test` | super admin | `monita-dev-password` |
| `tl@monita.test` | tech lead | `monita-dev-password` |

`dev.sql` also creates the project `demo`, owned by the tech lead. `qa.sql` creates no project: each e2e test creates its own.
