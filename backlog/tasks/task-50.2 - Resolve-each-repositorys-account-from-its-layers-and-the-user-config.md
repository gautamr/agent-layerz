---
id: TASK-50.2
title: Resolve each repository's account from its layers and the user config
status: To Do
assignee: []
created_date: '2026-11-21T17:57:00Z'
updated_date: '2026-11-21T18:10:00Z'
labels:
  - area:accounts
  - area:cli
milestone: m-3
dependencies:
  - TASK-50.1
parent_task_id: TASK-50
priority: high
ordinal: 18390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The client layer names the expected account; a machine-local user config can override it per folder; nothing is committed. Same pattern as commit_identity (decision-13).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A layer can declare an account name, and alz lint rejects anything in that field other than a name (no paths, no secrets)
- [ ] #2 alz account which prints the resolved account per target and the source that decided it (layer, user config or default); tests cover each source and the missing-folder fallback
- [ ] #3 The user config lives outside every repository, and alz build never writes an account value into a generated file
- [ ] #4 An account named by a layer takes effect only after alz account allow in that repository, which records the repository path in the user config; until then resolution skips it and prints the allow command once, and accounts set in the user config need no approval (tests: a fresh clone whose stack names an account starts on the default account; after allow it uses the named one)
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
2026-11-21: the owner approved the allow step (criterion #4). Reason: repository content is untrusted, and a manifest can name an external layer that names one of the owner's accounts (LAYERING-SPEC §7.12 'Trust'). Design in TASK-50.1's notes.
<!-- SECTION:NOTES:END -->
