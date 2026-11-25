---
id: TASK-115
title: Verify the Codex facts the autonomy mapping needs
status: To Do
assignee:
  - '@architect'
created_date: '2026-11-25T19:06:00Z'
labels:
  - area:autonomy
  - area:facts
milestone: m-8
dependencies: []
references:
  - docs/target-facts.md
  - TASK-18
priority: medium
ordinal: 216390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
doc-7 §12 lists the Codex facts behind the autonomy mapping (approval policy, sandbox mode from a trusted project, a choice tool). Some need project trust, which the skill forbids setting in a real account; record those as DOCS or UNVERIFIED with the reason.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 docs/target-facts.md has a row with a status for each Codex item XF1, XF3, XF4, XF7, XF8 and XF9 of doc-7 §12 (grep the IDs)
- [ ] #2 Each row says how it was checked, and the header's Codex version and date are updated (grep)
<!-- AC:END -->
