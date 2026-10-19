<!--
Fill in the three sections below. A sentence or two each is enough for a small change.
-->

## What

<!-- What this pull request changes, in plain words. Link the issue it closes: "Closes #12". -->

## Why

<!-- The symptom a user saw and its cause, or the need behind a feature. Not a restatement of the diff. -->

## How

<!-- The files to read first, and any choice the diff does not explain. -->

### Checklist

- [ ] `make check` passes
- [ ] `make e2e` passes, or no page or API changed
- [ ] A test covers the change; a bug fix has a test that fails without it
- [ ] A new migration is a new file; no merged migration changed
- [ ] `docs/api-specs/` and its tracker match any route that changed
- [ ] `docs/guide/` and `CHANGELOG.md` (under `## [Unreleased]`) describe any change users see
