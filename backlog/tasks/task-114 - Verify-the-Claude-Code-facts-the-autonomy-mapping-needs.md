---
id: TASK-114
title: Verify the Claude Code facts the autonomy mapping needs
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
priority: high
ordinal: 215390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
doc-7 §12 lists the Claude facts behind the autonomy mapping; most are UNVERIFIED. Adapters may write nothing on them until docs/target-facts.md records them. Use the verify-target-fact skill; never write to account folders.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 docs/target-facts.md has a row with a status for each Claude item CF1, CF3, CF4, CF5, CF6, CF9, CF10 and CF12 of doc-7 §12 (grep the IDs)
- [ ] #2 Each row says how it was checked (SMOKE command or DOCS URL), and the header date and Claude Code version are updated (grep)
<!-- AC:END -->
