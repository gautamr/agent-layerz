---
id: TASK-45
title: Keep each client's Claude memory in its own folder
status: To Do
assignee: []
created_date: '2026-11-21T17:40:00Z'
labels:
  - area:memory
  - area:security
milestone: m-3
dependencies:
  - TASK-44
priority: medium
ordinal: 11390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Memory is per folder path, but every client's memory lives in one shared projects/ folder, and nothing stops a note learned for one client from being read in another client's session. Ambiguous notes should be held, never routed across clients. Source: a design review (2026-11-21) of a cross-tool memory store for AI coding tools; this task keeps one of its good ideas.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Repositories under two different client roots write memory to two different folders (test with two scratch repositories)
- [ ] #2 The doctor chosen in TASK-44 reports a repository whose memory folder is outside its client's folder
- [ ] #3 Moving existing memory backs it up first and keeps every file (file counts before and after are in the notes)
<!-- AC:END -->
