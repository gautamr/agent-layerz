---
id: TASK-87
title: Give read-only commands a read-only view of the repository
status: To Do
assignee: []
created_date: '2026-11-24T06:35:00Z'
labels:
  - area:rust
  - area:security
milestone: m-7
dependencies: []
priority: low
ordinal: 158390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
R5 review (2026-11-24): check, diff, ls, explain and doctor open RepoRoot (the write capability) only to walk files; doctor's observe_* take raw paths. Add a read-only repository view (doc-6) so the types show these commands cannot write.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 check, diff, ls, explain and doctor hold no write capability, and doctor reads only through a capability
<!-- AC:END -->
