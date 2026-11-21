---
id: TASK-2
title: Create the GitHub repository and push only main
status: To Do
assignee:
  - '@gautamr'
created_date: '2026-11-19T17:12:00Z'
labels:
  - owner:human
  - area:repo
milestone: m-0
dependencies: []
priority: high
ordinal: 20
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The npm package page already links to github.com/gautamr/agent-layerz, which does not exist yet. Local backup branches hold history that must never be published (decision-3).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Public repository github.com/gautamr/agent-layerz exists
- [ ] #2 Only main is on the remote: `git ls-remote --heads origin` lists no backup or feature branches
- [ ] #3 The repository link on the npm package page opens the repository
<!-- AC:END -->
