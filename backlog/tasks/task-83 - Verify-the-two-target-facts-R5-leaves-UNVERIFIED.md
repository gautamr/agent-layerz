---
id: TASK-83
title: Verify the two target facts R5 leaves UNVERIFIED
status: To Do
assignee: []
created_date: '2026-11-23T18:30:00Z'
updated_date: '2026-11-25T15:34:00Z'
labels:
  - area:docs
milestone: m-7
dependencies: []
priority: medium
ordinal: 154390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Spec §8 (R5.1): alz emits exactly what the reference emits and nothing new on a fact marked UNVERIFIED in docs/target-facts.md, and each such row gets a docs task. Verify the two rows the spec names (a relative additionalDirectories entry, and how Claude treats an unknown settings key; TASK-79 may make the second moot) with the verify-target-fact skill, at SMOKE level under a temporary CLAUDE_CONFIG_DIR, and update the rows.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Both rows are recorded at DOCS level at least, SMOKE where possible
- [ ] #2 Where a row changes what alz may emit, a follow-up task exists
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
From TASK-97 (2026-11-25, Claude Code 2.1.323, isolated HOME): unknown settings keys loaded silently, and a settings.json with a wrongly typed permissions list did not load ('Expected array, but received number'). This bears on the UNVERIFIED row 'Claude, unknown settings key, malformed settings.json'; verifying it also changes the test that pins that row.
<!-- SECTION:NOTES:END -->
