---
id: TASK-55.5.13
title: 'R4.13: Who may declare: the shared rule of commit_identity and account'
status: In Progress
assignee:
  - '@implementer'
created_date: '2026-11-23T12:28:00Z'
updated_date: '2026-11-23T15:04:00Z'
labels:
  - area:rust
  - area:merge
  - area:security
milestone: m-7
dependencies:
  - TASK-55.5.12
parent_task_id: TASK-55.5
priority: high
type: task
ordinal: 126390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Plan for this step (§7.12, §7.13, decision-13).
- alz-core `merge/declarers.rs`: Declarer facts per layer (position in the stack, kind, visibility, Placement) and evaluate -> Declared<T> = None | Winner { layer, value } | Conflict, with the refused layers and their reasons (inside the repository; kind not client; public). The first layer, or one private kind: client layer, both outside the repository, may declare; the client's whole value wins; several clients, or a refused first layer, give Conflict. Build errors: one per refused layer and one naming every client when there are several.
- alz-io: Placement of each resolved layer (inside or outside the repository by its real path, a case variant of an in-repo path and a link into the repository counting as inside, as §7.12 says); reuse what R3 computes where it exists.
- The consumers (git config, alz account which and allow) are R6's.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The who-may-declare cases of tests 26 and 29 have Rust tests with the same verdict over in-memory stacks: repo layer, group layer, public client, second client, first layer inside the repository, confidential client, and no declaration
- [ ] #2 Placement counts a path inside the repository, a case variant of one and a link into it as inside (alz-io integration test)
- [ ] #3 Proptest: there is at most one winner, and a winner is the first layer or a private client layer outside the repository
<!-- AC:END -->
