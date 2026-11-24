---
id: TASK-55.7.9
title: 'R6.9: Account resolution as a pure decision, and the text of alz account which'
status: To Do
assignee: []
created_date: '2026-11-24T08:16:00Z'
labels:
  - area:rust
  - area:accounts
milestone: m-7
dependencies:
  - TASK-55.7.8
parent_task_id: TASK-55.7
priority: high
type: task
ordinal: 168390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Plan for this step (§7.13 "Resolution", "A named account that cannot apply fails closed", "Values set outside alz"; doc-6 §3.2).
- alz-core account/resolution.rs: per target, first match wins: the use choice, the user config's longest folders entry, the layer when allowed and readable, the default account. A value without its marker, or with another, is foreign and replaces steps 3 to 5, never 1 and 2; a set but empty value is foreign; a marker without its variable is stale.
- Applied { Folder, Default, Kept(Foreign) } and FailsClosed (missing folder, cache miss with the recorded account, changed, source gone, conflict, unreadable manifest, configured folder, invalid name, relative home), matched exhaustively; what the hook applies and the fix for each.
- account/which_report.rs: the lines of R6.2; the exit code (0 when every target resolves, 1 when any fails closed).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Property: a named account that cannot apply never resolves to the default account or to a newly declared account (proptest over generated facts)
- [ ] #2 Property: the first step that names an account wins, a foreign value replaces steps 3 to 5 but never 1 and 2, and a target the allowed entry does not name is skipped without failing (proptest)
- [ ] #3 Property: no report line holds a control or format character or a name that breaks the name rule, whatever the paths and sources hold (proptest)
<!-- AC:END -->
