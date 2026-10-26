# Changelog

Each project can publish release notes. The team writes them on the **Changelog** tab. Visitors read them on the public **Changelog** tab of the status page.

## Write a release

1. In a project, open **Changelog** and select **New release**. A side panel opens.
2. Enter the **Version**, for example `1.4.0`.
3. Pick **Released on**. It starts at today's date.
4. Enter a **Title**, up to 120 characters.
5. Under **Added**, **Changed**, **Fixed** and **Removed**, write one change per box. Select **Add a change** for more boxes. Leave a kind empty if it has no changes.
6. Select **Save release**.

| Field | Rule |
| --- | --- |
| Version | 1 to 40 characters: letters, digits, `.`, `_` or `-`. It must start with a letter or digit. Each version is unique in a project. |
| Title | 1 to 120 characters |
| Each kind | Up to 30 changes of up to 500 characters |

The word `latest` is not a valid version. The public address uses it to mean the newest release.

Monita shows every bad field at once. A change that wraps onto a second line is still one change.

## The list

The **Changelog** page lists releases newest first, 25 a page. It shows the version, title, release date and the number of changes. **Search releases** finds a version or a title. Sort by **Released**.

Select a row to edit it in the same panel. Select **Delete release** in the panel to remove it. A deleted release leaves the public page at once.

## Show it to visitors

A new project has the public Changelog tab on. Until you publish a release, the tab reads "No release is published yet." A project made before this default keeps its own setting. You can turn the tab off in **Settings**, **Status page**, under **Public tabs**. See [Status page](status-page.md#tabs).

While the tab is off, its address and its JSON answer 404.

## The public tab

- The page shows one release at a time. The newest opens first.
- A version index sits beside it. On a phone it is a select. **Find a version or title** narrows it. **Older releases** loads more.
- **Newer** and **Older** links under the release move one step.
- Each release has its own address: `/status/<slug>/changelog/<version>`. The address `/status/<slug>/changelog` shows the newest.
