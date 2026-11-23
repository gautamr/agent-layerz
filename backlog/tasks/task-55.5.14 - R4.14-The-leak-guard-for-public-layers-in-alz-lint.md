---
id: TASK-55.5.14
title: 'R4.14: The leak guard for public layers in alz lint'
status: To Do
assignee:
  - '@implementer'
created_date: '2026-11-23T12:28:00Z'
labels:
  - area:rust
  - area:security
milestone: m-7
dependencies:
  - TASK-55.5.13
  - TASK-55.5.1
parent_task_id: TASK-55.5
priority: high
type: task
ordinal: 127390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Plan for this step (§9.2); its scope follows owner question 2 of TASK-55.5. For a public layer, every file of LayerContents is checked against the layer's forbid_patterns, the secret scan of R4.11, and the email and private-host heuristics (warnings). forbid_patterns need a regex engine: run the dependency intake check first (versions, licence, features, cargo deny), propose the decision, and wait for the owner before any Cargo.toml change. Diagnostics name the file and the pattern's field, never the matched text.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Test 10's forbidden-pattern and fake-token cases pass against the Rust binary (mise run test:acceptance with ALZ_BIN on test 10)
- [ ] #2 A forbid_patterns entry that does not compile is a layer error naming its field, and no diagnostic holds matched text (unit tests)
- [ ] #3 Any new crate has an accepted decision and passes cargo deny (mise run rust:ci)
<!-- AC:END -->
