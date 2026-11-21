---
id: TASK-28.2
title: >-
  Add the tui-designer agent, two custom skills, the precedence rule and
  board-ordered next-task
status: To Do
assignee:
  - '@architect'
created_date: '2026-11-19T18:00:00Z'
updated_date: '2026-11-20T16:59:00Z'
labels:
  - area:lens
  - area:skills
  - area:repo
  - enabler
milestone: m-5
dependencies:
  - TASK-28.1
documentation:
  - backlog/docs/doc-2 - Agent-Lens-plan.md
parent_task_id: TASK-28
priority: high
ordinal: 70
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The epic needs a terminal UI specialist, project-specific skills for mods and terminal layout, a rule for how community and custom skills combine (decision-6), parallel implementers, and /next-task following the board order the owner sees.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A tui-designer agent exists in .layer/agents with the tui-design skill preloaded
- [ ] #2 Custom skill claude-code-mod covers the mod folder layout, claude plugin validate and test, the state contract and the verified Claude Code version range
- [ ] #3 Custom skill tui-design covers layout at 80, 110 and 144+ columns, truncation, colour plus symbols and a refresh budget
- [ ] #4 The repo instructions state the precedence rule: repo instructions and custom skills decide what and where; community skills guide how
- [ ] #5 The implementer agent works in an isolated git worktree
- [ ] #6 mise run alz:build, alz:check, lint and tests pass
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Board-ordered /next-task moved to task-30.4 (Delivery kit).
<!-- SECTION:NOTES:END -->
