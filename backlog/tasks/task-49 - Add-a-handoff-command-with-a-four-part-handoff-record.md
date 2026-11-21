---
id: TASK-49
title: Add a /handoff command with a four-part handoff record
status: To Do
assignee: []
created_date: '2026-11-21T17:40:00Z'
labels:
  - area:memory
  - area:skills
milestone: m-6
dependencies: []
priority: medium
ordinal: 15390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
FRAMEWORK-GAPS G13 plans a /handoff command so a session on another account or tool continues without a re-brief. A fixed record makes handoffs comparable. Commands stay thin entry points and knowledge goes in skills (decision-7). Source: a design review (2026-11-21) of a cross-tool memory store for AI coding tools; this task keeps one of its good ideas.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The personal layer has a /handoff command that writes four parts: what was done, files touched, open items, next step; on a board task it appends to the task notes, otherwise it writes docs/handoff.md
- [ ] #2 G13's acceptance test is run once and recorded: a task started on one account continues on another (claude-as) from the handoff without a re-brief
- [ ] #3 delivery-protocol's handover section points to the command instead of repeating it
<!-- AC:END -->
