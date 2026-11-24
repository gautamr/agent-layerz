---
id: TASK-55.9.4
title: State in the spec and target facts only what alz does
status: To Do
assignee:
  - '@architect'
created_date: '2026-11-24T17:44:00Z'
updated_date: '2026-11-24T17:49:00Z'
labels:
  - area:spec
  - area:docs
milestone: m-7
dependencies:
  - TASK-55.9.1
parent_task_id: TASK-55.9
priority: medium
ordinal: 186390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
docs/LAYERING-SPEC.md still describes the port: §13's reference drift list and about 175 lines naming the TypeScript alz, the reference, reference drift, parity, Node or port phases. Under decision-39 the spec states only what alz does, and the later subtasks write Rust tests from it, so the text must be final before the coverage matrix is built. First the user-visible differences from the published 0.1.0 that the drift list names move to CHANGELOG.md's Unreleased section in neutral words, so the first Rust release can describe them. docs/target-facts.md is reworded the same way (its facts about Claude Code, Codex and git stay). §13's fixture path becomes fixtures/. This is an editorial change of the contract: no behaviour, layer format or generated output changes, so it needs no version bump by itself. Out of scope: new rules (a spec hole found later is its own task).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 grep -nE 'TypeScript|[Tt]he reference|reference drift|referenceDrift|parity|Node|npm|vitest|Rust .alz' docs/LAYERING-SPEC.md docs/target-facts.md prints nothing, or only lines the notes list as not about an implementation (grep)
- [ ] #2 §13 has no drift list, each item states only behaviour, and its fixture sentence names fixtures/ (spec diff)
- [ ] #3 CHANGELOG.md Unreleased states each user-visible difference from 0.1.0 that the drift list named (diff review)
- [ ] #4 mise run rust:check passes
<!-- AC:END -->
