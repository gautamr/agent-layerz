---
id: TASK-119
title: Write the autonomy mapping into Claude's settings and CLAUDE.md
status: To Do
assignee:
  - '@implementer'
created_date: '2026-11-25T19:07:00Z'
updated_date: '2026-11-25T19:14:00Z'
labels:
  - area:autonomy
  - area:adapter
milestone: m-8
dependencies:
  - TASK-118
  - TASK-112
priority: medium
ordinal: 220390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
doc-7 §9 and §11: each level maps to permissions.defaultMode, sandbox keys and floor keys, and each class's rules go to allow, ask or deny by its row. Only keys B1 recorded may be written.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A golden .claude/settings.json per level holds the keys of doc-7 §9 as the spec states them (tests)
- [ ] #2 No level writes defaultMode bypassPermissions or auto, and every never-class rule is in permissions.deny at every level (proptest)
- [ ] #3 Each class's rules are in allow, ask or deny according to its row at the effective level (tests)
- [ ] #4 The facts harness passes with Claude's NO_FACT_RECORDS unchanged, and CLAUDE.md names AskUserQuestion only if target-facts records it (test and grep)
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Owner (2026-11-26): decision-40 accepted. When an adapter cannot express a security item (for example a decision class's rules on a target without allow/ask/deny lists), alz fails the build with an error naming the item and the target, never a warning; a layer may opt that target out of the item explicitly.
<!-- SECTION:NOTES:END -->
