---
id: TASK-55.1
title: 'R0: Rust foundation and recorded choices'
status: In Progress
assignee: []
created_date: '2026-11-22T12:54:00Z'
updated_date: '2026-11-22T16:31:00Z'
labels:
  - area:rust
  - area:spec
milestone: m-7
dependencies: []
references:
  - decision-23
  - decision-24
  - decision-25
  - decision-26
  - decision-27
parent_task_id: TASK-55
priority: high
ordinal: 33390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
doc-5 phase R0: repository layout, toolchain through mise, the YAML library spike, git access, MSRV and targets, output-text policy. Each choice becomes a decision.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A Cargo workspace builds an alz binary that prints its version and help; the five choices in doc-5 are recorded as decisions
- [ ] #2 The Rust standard in doc-5 is written as a .layer rule for Rust code and enforced in CI from the first commit: rustfmt, clippy -D warnings with pedantic and the chosen restriction lints, forbid(unsafe_code), cargo deny and cargo audit
- [ ] #3 The crate layout is designed from the spec as Rust and recorded as a decision before any domain code; no module mirrors src/ because src/ had it
- [ ] #4 Every R0 subtask is Done
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
2026-11-22: proposed layout, crate split and type-level design are in doc-6; the Rust standard is the personal skill gr-rust (agent-layer-personal, uncommitted until the owner releases it). R0 records the layout as a decision, adopting or changing doc-6.

2026-11-22: the personal layer folder and repository are renamed agent-layerz-personal, and the planned starter is agent-layerz-starter; earlier notes keep the old names.

2026-11-22: split into small subtasks (spike, decisions, skeleton, tooling, CI). The .layer Rust rule, Rust-aware agents, rust:check and the cargo stop hook were done in commit 23ca45e, which covers the rule half of AC #2; CI enforcement is the CI subtask.

2026-11-22: R0 choices 1, 3, 4, 5 and the directory-access crate are recorded as decision-23 to decision-27 (TASK-55.1.2). The YAML library (TASK-55.1.1) is still open.
<!-- SECTION:NOTES:END -->
