---
id: TASK-46
title: Promote a lesson from memory into a layer only through a review list
status: To Do
assignee: []
created_date: '2026-11-21T17:40:00Z'
labels:
  - area:memory
  - area:skills
milestone: m-6
dependencies: []
priority: medium
ordinal: 12390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Lessons pile up in agent memory and session feedback. The retrospective skill turns repeated corrections into layer tasks, but nothing collects memory candidates or remembers which were rejected. Related: TASK-19 (alz promote). Source: a design review (2026-11-21) of a cross-tool memory store for AI coding tools; this task keeps one of its good ideas.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The retrospective skill builds a review list of candidate lessons from agent memory and session feedback, each with its source and date, and changes no layer itself
- [ ] #2 A lesson that names a client, a client project or a client path is never proposed for the personal layer; it goes to that client's layer or is held, and alz lint passes on every accepted change
- [ ] #3 A rejected lesson is recorded so the next retrospective does not propose it again
<!-- AC:END -->
