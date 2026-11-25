---
id: TASK-123
title: Host third-party adapters as WASM components with no imports
status: To Do
assignee:
  - '@implementer'
created_date: '2026-11-25T19:07:00Z'
labels:
  - area:adapter
  - area:security
milestone: m-8
dependencies:
  - TASK-122
references:
  - decision-40
priority: low
ordinal: 224390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
decision-40 point 8: a component cannot reach the file system, network or clock, and its CPU and memory are bounded, so a third-party adapter is deterministic and safe. Needs a security review.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A component that imports anything is refused at load with the error 'target' (test)
- [ ] #2 A looping component stops at its fuel or epoch limit and a greedy one at its memory limit, each with the error 'target' (tests)
- [ ] #3 The same component and model give identical bytes twice, and component output goes through the core checks of the output-check task (tests)
<!-- AC:END -->
