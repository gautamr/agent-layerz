---
id: TASK-101
title: Warn about a hook event a target does not support, and list the fallbacks
status: To Do
assignee: []
created_date: '2026-11-25T16:34:00Z'
labels:
  - area:rust
milestone: m-7
dependencies: []
priority: low
ordinal: 203390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Spec §7.5 says a hook event a target does not support becomes a warning and the report lists the fallbacks; no code does this. Owner (2026-11-25): mark it planned in the spec; build it later.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A hook on an event a target does not support gives the warning and the fallbacks of spec §7.5 (Rust test)
<!-- AC:END -->
