---
id: TASK-55.1
title: 'R0: Rust foundation and recorded choices'
status: To Do
assignee: []
created_date: '2026-11-22T12:54:00Z'
updated_date: '2026-11-22T14:54:00Z'
labels:
  - area:rust
  - area:spec
milestone: m-7
dependencies: []
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
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
2026-11-22: proposed layout, crate split and type-level design are in doc-6; the Rust standard is the personal skill gr-rust (agent-layer-personal, uncommitted until the owner releases it). R0 records the layout as a decision, adopting or changing doc-6.

2026-11-22: the personal layer folder and repository are renamed agent-layerz-personal, and the planned starter is agent-layerz-starter; earlier notes keep the old names.
<!-- SECTION:NOTES:END -->
