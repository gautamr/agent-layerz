---
id: TASK-55.3.10
title: >-
  R2.10: alz lint of a layer and a stack manifest, through a no-follow capped
  read
status: In Progress
assignee:
  - '@implementer'
created_date: '2026-11-23T05:18:00Z'
updated_date: '2026-11-23T06:37:00Z'
labels:
  - area:rust
  - area:cli
milestone: m-7
dependencies:
  - TASK-55.3.6
  - TASK-55.3.7
parent_task_id: TASK-55.3
priority: high
type: task
ordinal: 99390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Plan for this step. Spec §10 `alz lint [--layer <path>]`, R2.1's "Output and exit codes", decision-27 for the read.

- alz-io: `crates/alz-io/src/fs.rs` with `fs/read_only_dir.rs`. `ReadOnlyDir::open(&Path)` for a directory the person named (`--layer` or the working directory) and `ReadOnlyDir::read_file(&SafeRelPath, cap)`: one component, `O_RDONLY | O_NONBLOCK | O_CLOEXEC | O_NOFOLLOW`; `fstat` on that handle must show `S_ISREG`, else refused; at most cap + 1 bytes read, and the file refused when the last byte arrives (§5.3: `layer.yaml` is a regular file, never a link or a folder). Adds rustix 1.1.6 or later with `fs` (decision-27). The walk is R3's. alz-io depends on alz-core for `SafeRelPath` and turns `ReadError` into a diagnostic.
- Binary: `crates/alz/src/command.rs` with `command/lint.rs`. `--layer <path>` lints that layer's `layer.yaml` (R2.5). Without it, a stack manifest in the working directory is linted as the manifest plus `layer.lock` when present (R2.6, R2.7); any other directory as a layer. Output as R2.1 states.
- R2 scope: lint checks only what R2 models. Instructions, skills, agents, settings, protected variables, the leak guard, the secret scan and slots arrive with R4 and R5 (R5 owns `lint`), and stack mode stops after a valid manifest and lock with a fixed `error [not-ported]` that names R3, exit 1 (owner answer 5); R3 removes it.

Tests: assert_cmd with files set up through alz-test-support; `layer.yaml` as a link, a folder, a FIFO, past 1 MiB and as invalid UTF-8, each refused without blocking.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 alz lint --layer <dir> exits 0 with an empty stderr for a valid layer.yaml, and 1 with error [layer] lines for a schema error (assert_cmd)
- [ ] #2 A layer.yaml that is a link, a folder or a FIFO is refused without blocking, and one past 1 MiB is refused after reading at most 1 MiB + 1 byte (alz-io tests)
- [ ] #3 alz lint in a directory with a stack manifest reports manifest and lock errors, and with a valid manifest and lock exits 1 with error [not-ported] naming R3 (assert_cmd; owner answer 5)
<!-- AC:END -->
