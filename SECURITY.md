# Security Policy

## Supported versions

Only the latest release gets security fixes. Fixes are not backported to older versions. Upgrade with the steps in [Operations](docs/guide/operations.md).

| Version | Supported |
| :--- | :--- |
| Latest 1.x release | Yes |
| Older | No |

## Report a vulnerability

**Do not open a public issue for a security problem.**

Report it privately on GitHub: open the repository's **Security** tab and select **Report a vulnerability**, or go to [the advisory form](https://github.com/PT-Perkasa-Pilar-Utama/monita/security/advisories/new). Only the maintainers see the report.

Include:

- what the problem is and what an attacker gains;
- the steps or a small proof of concept that shows it;
- the Monita version (`monita --version`) and how it runs (behind a proxy or not, `MONITA_ENV`);
- a fix, if you have one in mind.

You get an answer within **72 hours**. We assess a valid report within **7 days**, and we aim to publish a fix and an advisory within **30 days** of confirming it.

## Scope

Monita runs on your own server and holds your projects' uptime data, incidents, tickets and alert settings. In scope:

- **Public routes:** the status pages and their JSON, the ticket form, the ping URLs and `/api/v1`. Anyone can reach them.
- **Access control:** a tech lead who reads or changes another owner's project, a supervisor who changes anything, or a user who gains a role.
- **Secrets:** API keys, session and invite tokens and ref IDs are stored only as SHA-256 hashes; a raw value in the database, a log or a page is a bug.
- **Private data on a public page:** a ticket description or reporter, an update's author, Monita's own cause text, or a Triage ticket.
- **Outbound requests:** a way for a tech lead to read a monitored response or a webhook answer back beyond what the pages show.
- **Dependencies:** a vulnerable crate or package that ships in the binary.

Out of scope:

- Monitors that reach internal addresses. Checking internal services is what Monita is for (SEC-15 in the [security spec](docs/technical-specs/07-security.md)).
- Project secrets and the mail password are stored as entered in the database file, which only the `monita` user can read (SEC-23, SEC-25). A report that needs read access to that file is out of scope.
- A setup that ignores the install guide: Monita without HTTPS in front, or `MONITA_TRUST_PROXY=true` on a port that other hosts can reach.
- Attacks that need root or write access on the server.
- Build-time tools that do not ship in the binary.
- Questions and feature requests: open an issue instead.

## How releases are protected

- Each release archive comes with a SHA-256 in `checksums.txt` and a build-provenance attestation. Check one with `gh attestation verify monita_linux_amd64.tar.gz --repo PT-Perkasa-Pilar-Utama/monita`.
- CI checks every crate for known advisories and licences with `cargo deny`.
- Every GitHub Action is pinned by commit SHA, and Dependabot proposes updates each month.
- The [security spec](docs/technical-specs/07-security.md) lists each rule (SEC-01 to SEC-26) and where the code enforces it.
