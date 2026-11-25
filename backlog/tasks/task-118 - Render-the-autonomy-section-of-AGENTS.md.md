---
id: TASK-118
title: Render the autonomy section of AGENTS.md
status: To Do
assignee:
  - '@implementer'
created_date: '2026-11-25T19:07:00Z'
labels:
  - area:autonomy
milestone: m-8
dependencies:
  - TASK-117
priority: medium
ordinal: 219390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
doc-7 §11: the effective level, its ceiling and the decision table with the how-to-ask text of the level, in the instructions every target reads.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A golden test per level shows the level, the ceiling and their layers, the four rows with their classes, and the how-to-ask text of doc-7 §11
- [ ] #2 The Never row holds every never class at every level (proptest)
- [ ] #3 The section counts against instructions_max_lines (test over the budget), and a stack without autonomy.yaml has no section (test)
<!-- AC:END -->
