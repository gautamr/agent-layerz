---
id: TASK-55.5.9
title: 'R4.9: Permissions with the floor'
status: To Do
assignee:
  - '@implementer'
created_date: '2026-11-23T12:27:00Z'
labels:
  - area:rust
  - area:merge
  - area:security
milestone: m-7
dependencies:
  - TASK-55.5.8
parent_task_id: TASK-55.5
priority: high
type: task
ordinal: 122390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Plan for this step (§7.6).
- item/permission.rs: allow, ask and deny as ListEdit of neutral rules (a plain YAML list adds; `+[..]`, `-[..]` and `[..]` strings edit, as the reference reads them); default_mode a PermissionMode ordered by strictness; floor: true needs a default_mode.
- merge: lists combine as OrderedSet. A more specific layer whose deny edit drops a rule of a more general layer is a `permission-floor` error naming the layer, the rule (stripped) and the layer that set it; the rule stays. default_mode: most specific wins, never looser than the strictest floor of a more general layer.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Test 07's cases have Rust tests with the same verdict: a repo layer removing a personal deny fails naming the layer and the rule, and allow, ask and deny combine (unit tests)
- [ ] #2 Proptest: merging a further layer either refuses or keeps every deny rule of the stack, and the merged default_mode is never looser than a floor
- [ ] #3 Unknown fields, an unknown mode and floor without default_mode are permissions errors (unit tests)
<!-- AC:END -->
