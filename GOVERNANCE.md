# Governance

This document names who maintains Monita, what each role may do, and how a contributor gets more access.

## Maintainers

Monita is maintained by [PT. Perkasa Pilar Utama](https://github.com/PT-Perkasa-Pilar-Utama).

| Role | GitHub | Responsibilities |
| :--- | :--- | :--- |
| Lead maintainer | [@snowfluke](https://github.com/snowfluke) | Releases, security response, final review, repository and organisation settings |
| Maintainer | [@amaruki](https://github.com/amaruki) | Review and merge pull requests, cut releases |
| Maintainer | [@saikanov](https://github.com/saikanov) | Review and merge pull requests, cut releases |

[CODEOWNERS](.github/CODEOWNERS) asks every maintainer to review each pull request. Any one of them may approve it.

## Roles and permissions

| Role | Access | Granted to |
| :--- | :--- | :--- |
| Contributor | Fork and open pull requests | Anyone |
| Triager | Label and sort issues | Trusted contributors, on a maintainer's invitation |
| Maintainer | Review and merge pull requests, cut releases | By the lead maintainer's decision |

The sensitive resources are the repository's admin settings, its GitHub Pages site, the release workflow and the organisation membership. Releases are built and attested by CI, never on a person's machine. Only the lead maintainer changes these resources.

## Granting more access

Merge rights are not automatic. Before a contributor becomes a maintainer, the lead maintainer reviews their record: merged pull requests, the quality of their reviews, and how they follow [CONTRIBUTING.md](CONTRIBUTING.md). The change is recorded by a pull request that updates this file, so the decision is reviewed and stays on record. Access is removed the same way when someone steps back.

## Decisions

Everyday changes go through the pull-request flow in [CONTRIBUTING.md](CONTRIBUTING.md). Disagreements are settled in the issue or the pull request. The lead maintainer has the final say on scope, the data model, security and release timing. A decision that is hard to reverse is written down in [docs/decisions/](docs/decisions/).

## Changing this document

Open a pull request against this file. The lead maintainer must approve it before it merges.
