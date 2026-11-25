---
id: TASK-122
title: Define the WIT world alz:adapter@1 that mirrors the contract
status: To Do
assignee:
  - '@implementer'
created_date: '2026-11-25T19:07:00Z'
labels:
  - area:adapter
milestone: m-8
dependencies:
  - TASK-121
references:
  - decision-40
priority: low
ordinal: 223390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The language-neutral form of the contract for components: describe and render exported, nothing imported. Waits for the runtime decision of the intake task.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 wit/alz-adapter.wit exports describe and render and has no import (grep)
- [ ] #2 A test converts model v1 and a sample output to the WIT types and back without loss
<!-- AC:END -->
