---
id: TASK-72
title: >-
  With HOME unset and no user database entry, account resolution fails closed
  instead of crashing
status: To Do
assignee: []
created_date: '2026-11-23T04:51:00Z'
labels:
  - discovered
  - area:rust
milestone: m-7
dependencies: []
references:
  - docs/LAYERING-SPEC.md
  - src/cli/io.ts
priority: low
type: bug
ordinal: 89390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Found in the TASK-65 review. src/cli/io.ts takes io.env.HOME ?? homedir(). Node's os.homedir() looks up the effective user id when HOME is unset, and throws when the user database has no entry, so the command crashes. Spec §7.13 'Environment' says that case counts as a HOME that is not an absolute path: both files are ignored with one warning, no account folder is set, and a named account fails closed. Fixed only in Rust, in R6 (TASK-55.7), per decision-30.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 With HOME unset and an effective user id that has no user database entry, alz account which exits 0, warns once that HOME is not an absolute path, and reports the default account for every target (Rust test, since the setup needs a uid without an entry)
<!-- AC:END -->
