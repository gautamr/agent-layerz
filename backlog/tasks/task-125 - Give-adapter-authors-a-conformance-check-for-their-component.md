---
id: TASK-125
title: Give adapter authors a conformance check for their component
status: To Do
assignee:
  - '@implementer'
created_date: '2026-11-25T19:07:00Z'
labels:
  - area:adapter
  - area:cli
milestone: m-8
dependencies:
  - TASK-123
references:
  - decision-40
priority: low
ordinal: 226390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Third parties cannot run the built-in adapters' CI. One command runs the core's output checks and the facts harness against a component and a coverage stack.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 alz adapter check <component> runs the output checks and the facts harness and exits non-zero on any failure (CLI tests with a passing and a failing component)
- [ ] #2 The guide for adapter authors documents the command (grep)
<!-- AC:END -->
