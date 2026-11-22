---
id: TASK-56.5
title: Optional enforcement of the review gate
status: To Do
assignee: []
created_date: '2026-11-22T13:49:00Z'
updated_date: '2026-11-22T13:50:00Z'
labels:
  - area:process
  - area:security
milestone: m-3
dependencies:
  - TASK-56.4
parent_task_id: TASK-56
priority: low
ordinal: 47390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Design and build an opt-in check that the gate held, without trusting agent-written text blindly: for gate: done, a check (hook or alz command) that refuses to mark a task Done unless its notes record a review by each required reviewer for the unit; for gate: commit, a commit-msg or pre-commit hook that refuses a feat or fix commit for a task whose unit has no recorded review. Define the review record format (reviewer, verdict, date, findings fixed or recorded). State the honest limit: an agent can write a fake record; CI branch protection is the strong gate once pull requests exist.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The review record format is in the spec, and the check accepts a complete record and refuses a missing or partial one, naming the missing reviewer
- [ ] #2 With enforcement off (the default) nothing is checked; with it on, the check is a generated hook that a later layer cannot remove when it is locked
<!-- AC:END -->
