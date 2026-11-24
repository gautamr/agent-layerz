---
id: TASK-85
title: Choose the claude program only from PATH entries the repository cannot set
status: To Do
assignee: []
created_date: '2026-11-24T06:34:00Z'
labels:
  - area:rust
  - area:security
milestone: m-7
dependencies: []
priority: medium
ordinal: 156390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
R5 security review (2026-11-24): PATH entries added by a repository-local mise or direnv configuration are absolute, so a repository can supply the claude program that alz doctor and alz ls --live run (it needs mise trust or direnv allow). Treat PATH and ALZ_CLAUDE_BIN as the threat model treats XDG_* under __MISE_DIFF / DIRENV_DIFF: refuse or warn on a program that lies inside the repository.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A claude program inside the repository is refused or warned about, with tests
<!-- AC:END -->
