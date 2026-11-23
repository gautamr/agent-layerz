---
id: TASK-55.6.8
title: 'R5.8: alz build and alz check'
status: To Do
assignee:
  - '@implementer'
created_date: '2026-11-23T18:09:00Z'
labels:
  - area:rust
  - area:cli
milestone: m-7
dependencies:
  - TASK-55.6.6
  - TASK-55.6.7
parent_task_id: TASK-55.6
priority: high
type: task
ordinal: 140390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Plan for this step: crates/alz/src/command/{pipeline,build,check}.rs.
- pipeline: read the manifest and the lock, StackResolver, placements_of_stack, merge, render, the AGENTS.override.md walk (alz-io repo_walk, no-follow, .git skipped), lock_of, observe, plan_workspace. Diagnostics printed once, through report.rs.
- build: collisions are `error [collision]` lines and exit 1 unless --adopt (R5.14); apply; the .gitignore block; the summary line of §10.
- check: the change, collision and lock lines of §10, then the failed or passed line; --skip-identity parsed; the identity comparison itself is R6 (a seam, no behaviour).
- Golden trees: a script under crates/alz/tools/ builds every fixture repository with the reference and writes its tree; a Rust integration test builds the same fixtures with the Rust alz and compares byte for byte, drift samples marked as in the YAML goldens.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 alz build writes, and alz check passes on, the reference's bytes for every fixture repository (golden-tree integration test)
- [ ] #2 Acceptance files 01, 02, 07 to 09, 11, 14, 15, 23 and 37's build rows pass against ALZ_BIN (vitest run with ALZ_BIN=target/release/alz, counts in the notes)
- [ ] #3 The build and check forms of TASK-52, TASK-53, TASK-63 and TASK-71 in tests 24 and 36 pass against ALZ_BIN (vitest run)
<!-- AC:END -->
