---
id: TASK-55.7.10
title: 'R6.10: alz account which'
status: To Do
assignee: []
created_date: '2026-11-24T08:16:00Z'
labels:
  - area:rust
  - area:accounts
  - area:cli
milestone: m-7
dependencies:
  - TASK-55.7.2
  - TASK-55.7.9
parent_task_id: TASK-55.7
priority: high
type: task
ordinal: 169390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Plan for this step (§10 `alz account which`, §7.13).
- cli.rs: `alz account which [--use <target>=<name>]...`; an unknown target, an invalid name, a target given twice or a positional argument is a usage error (exit 2); an unknown account subcommand exits 2, stripped.
- Flow: the §7.13 variables by value and the markers and mise or direnv by presence only, Home, Place (once), the root walk, the two files, the folder listing, the cached declaration only for an allowed root, the resolution, stdout lines and warnings on stderr. The command is handed no GitRunner, RepoRoot or StateDir.
- Rewrites test 24's escape case so the folder name comes from allowed.yaml (the Rust alz refuses the layer.yaml the old case wrote, §5.2).
- Deletes the entries of the cases it makes pass from test/ci/r6-pending.json in the same change.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Every case of tests 30 and 31, test 29's who-wins cases and test 24's three cases passes against ALZ_BIN, and their entries are gone from test/ci/r6-pending.json (vitest run; check-r6-pending exits 0)
- [ ] #2 In an allowed repository alz account which runs no git and changes no file (test 31's gitProxy and snapshot cases pass against ALZ_BIN)
- [ ] #3 An assert_cmd test shows exit codes 0, 1 and 2, and that warnings go to standard error only
<!-- AC:END -->
