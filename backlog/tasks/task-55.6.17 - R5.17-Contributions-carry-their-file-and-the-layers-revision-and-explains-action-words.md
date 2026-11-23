---
id: TASK-55.6.17
title: >-
  R5.17: Contributions carry their file and the layer's revision, and explain's
  action words
status: To Do
assignee: []
created_date: '2026-11-23T18:30:00Z'
labels:
  - area:rust
  - area:merge
milestone: m-7
dependencies:
  - TASK-55.6.2
parent_task_id: TASK-55.6
priority: high
ordinal: 153390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Found by R5.1 (2026-11-24): Contribution in crates/alz-core/src/merge/provenance.rs holds only the layer and an Action (Define, Extend, Replace, Remove). Byte-identical headers need each contribution's layer file and the layer's revision or version (test 26 asserts a source like <layer>@<version>:layer.yaml), and explain needs per-kind action words (test 17 asserts e.g. 'extend: paths +[e2e/**]'). Extend the merge result so every contribution carries its ItemPath (the layer file) and the layer's LayerLabel input, and each kind's merge records the detail explain prints, as typed values (no strings built in merge). Pure alz-core; property tests that provenance stays in stack order; parity with the reference's explain words for the fixtures of tests 17 and 26.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Every contribution names its layer file and the layer's revision or version, in stack order
- [ ] #2 Each kind's merge records the typed detail explain needs for the reference's action words (fixtures of tests 17 and 26)
- [ ] #3 mise run rust:ci passes
<!-- AC:END -->
