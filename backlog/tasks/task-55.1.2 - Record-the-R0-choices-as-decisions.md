---
id: TASK-55.1.2
title: Record the R0 choices as decisions
status: To Do
assignee: []
created_date: '2026-11-22T15:52:00Z'
updated_date: '2026-11-22T15:53:00Z'
labels:
  - area:rust
  - area:spec
milestone: m-7
dependencies:
  - TASK-55.1.1
parent_task_id: TASK-55.1
priority: high
ordinal: 60390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The owner decides; the architect drafts each with options and a recommendation, then fills the decision body after the owner answers (standing exception). Choices from doc-5: (1) repository layout and crate split (adopt or change doc-6); (2) the YAML library (from the spike); (3) git access: the git binary with argument arrays (recommended) or a library; (4) MSRV and target platforms; (5) message wording policy (the spec defines it; clap's conventions update the spec on purpose); (6) safe directory access crate (cap-std or rustix).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Six accepted decisions exist, one per choice, each with Context, Decision and Consequences filled
- [ ] #2 doc-5 and doc-6 are updated where a decision changes them
<!-- AC:END -->
