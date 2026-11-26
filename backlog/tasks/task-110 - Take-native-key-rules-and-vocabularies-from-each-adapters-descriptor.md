---
id: TASK-110
title: Take native key rules and vocabularies from each adapter's descriptor
status: In Progress
assignee:
  - '@implementer'
created_date: '2026-11-25T19:06:00Z'
updated_date: '2026-11-26T07:38:00Z'
labels:
  - area:adapter
milestone: m-8
dependencies:
  - TASK-108
references:
  - decision-40
priority: medium
ordinal: 211390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
item/native_keys.rs holds Claude and Codex rows in the core. decision-40 point 3 moves them into descriptors (Codex as a descriptor without a renderer until its adapter exists); the core keeps the floor algebra, so the security floor stays core law.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 item/native_keys.rs holds no Claude or Codex row and still holds Strictness and loosenings (grep)
- [ ] #2 settings.yaml validates a target section with that target's descriptor, and the existing settings tests pass unchanged
- [ ] #3 The laws of merge/settings/laws.rs pass for a test descriptor with a flag floor and a ladder floor (proptest)
- [ ] #4 The acceptance suite and mise run alz:check pass with unchanged output
<!-- AC:END -->
