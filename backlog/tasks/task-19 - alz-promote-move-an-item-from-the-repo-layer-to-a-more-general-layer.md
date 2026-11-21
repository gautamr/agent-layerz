---
id: TASK-19
title: 'alz promote: move an item from the repo layer to a more general layer'
status: To Do
assignee:
  - '@architect'
created_date: '2026-11-19T17:13:00Z'
labels:
  - area:cli
milestone: m-3
dependencies: []
priority: medium
ordinal: 310
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
After alz build --adopt in a client hub, agents shared by the sibling repositories had to be moved by hand from .layer/ into the hub's client layer.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 alz promote <type> <name> --to <layer path> moves agents, skills, commands, rules and instruction files
- [ ] #2 It refuses a target layer outside the working copy, and a public target layer whose leak guard would fail
- [ ] #3 A rebuild of the current repository gives the same output as before the move
- [ ] #4 Spec and README are updated
<!-- AC:END -->
