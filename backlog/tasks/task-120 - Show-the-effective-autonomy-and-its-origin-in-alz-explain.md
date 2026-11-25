---
id: TASK-120
title: Show the effective autonomy and its origin in alz explain
status: To Do
assignee:
  - '@implementer'
created_date: '2026-11-25T19:07:00Z'
labels:
  - area:autonomy
  - area:cli
milestone: m-8
dependencies:
  - TASK-117
priority: low
ordinal: 221390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
A level, a ceiling and class rows come from several layers; people need to see which layer decided what before they trust an autonomous run.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 alz explain autonomy prints the level, the ceiling, the layer of each and every class with its row (CLI test)
- [ ] #2 Text from layers is printed stripped (test with a control character)
<!-- AC:END -->
