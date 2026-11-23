---
id: TASK-55.6.12
title: 'R5.12: alz init'
status: To Do
assignee:
  - '@implementer'
created_date: '2026-11-23T18:09:00Z'
labels:
  - area:rust
  - area:cli
milestone: m-7
dependencies:
  - TASK-55.6.7
parent_task_id: TASK-55.6
priority: high
type: task
ordinal: 144390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Plan for this step (§10 init, §4.4).
- scaffold.rs (pure): the repo manifest, the repo layer's layer.yaml and instructions file, the layer scaffold, the mise tasks text, byte-identical to the reference; stack options parsed with the R2 source types (a bare path means path:, path:<dir>@<ref> pins a ref).
- --hub: the hub's layer/ pinned to its newest layer-v* tag through GitRunner (decision-24); the tag is parsed as a GitRef before it is written.
- Writes are create-only through RepoRoot; an existing layer.yaml is the error init; a name that is not a layer name is the error init.
- The commit identity set-up after scaffolding is R6 (TASK-55.7): R5 prints nothing for it.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 alz init --kind repo and --kind layer write the reference's bytes and the created lines (assert_cmd tests and test 21 against ALZ_BIN)
- [ ] #2 alz init --hub pins the hub layer to its newest layer-v* tag, and a tag that is not a valid ref is refused before anything is written (test 16's init cases and a unit test)
- [ ] #3 alz init never overwrites an existing file (alz-io test)
<!-- AC:END -->
