---
id: TASK-107
title: Render the Claude adapter from model v1 instead of Merged
status: To Do
assignee:
  - '@implementer'
created_date: '2026-11-25T19:06:00Z'
labels:
  - area:adapter
milestone: m-8
dependencies:
  - TASK-106
references:
  - decision-40
priority: medium
ordinal: 208390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The Claude adapter reads alz-core's internal Merged today. decision-40 point 2 has the core convert Merged into the published model and the adapter read only that, so merge types can change without breaking adapters.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 alz-core converts a Merged stack into Model, and one test per item kind of spec §7 shows each neutral field reaches the model
- [ ] #2 Nothing under crates/alz-core/src/target/claude imports crate::merge (grep)
- [ ] #3 Output is byte-identical: the acceptance suite and mise run alz:check pass
<!-- AC:END -->
