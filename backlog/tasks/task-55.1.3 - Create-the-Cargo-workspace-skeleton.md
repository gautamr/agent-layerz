---
id: TASK-55.1.3
title: Create the Cargo workspace skeleton
status: To Do
assignee: []
created_date: '2026-11-22T15:52:00Z'
updated_date: '2026-11-22T15:53:00Z'
labels:
  - area:rust
milestone: m-7
dependencies:
  - TASK-55.1.2
parent_task_id: TASK-55.1
priority: high
ordinal: 61390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
From the recorded layout: root Cargo.toml (resolver 3, workspace.package with edition 2024, rust-version and licence, workspace.dependencies, and the workspace.lints table from gr-rust references/layout-and-tooling.md), rust-toolchain.toml, clippy.toml, and the crates alz-core (no I/O; its clippy.toml bans std::fs, std::process and std::env), alz-io, alz (clap derive binary) and alz-test-support (publish = false). No domain code yet.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 alz --version and alz --help work from cargo run, and an assert_cmd test proves both
- [ ] #2 mise run rust:check passes with clippy pedantic and the restriction lints at -D warnings and no suppressions
- [ ] #3 A test or lint proves alz-core cannot call std::fs (for example a deliberate call fails clippy, recorded in the task notes and removed)
<!-- AC:END -->
