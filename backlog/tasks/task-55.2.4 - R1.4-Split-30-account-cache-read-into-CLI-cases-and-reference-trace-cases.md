---
id: TASK-55.2.4
title: 'R1.4: Split 30-account-cache-read into CLI cases and reference trace cases'
status: To Do
assignee: []
created_date: '2026-11-22T17:55:00Z'
labels:
  - area:test
  - area:rust
milestone: m-7
dependencies:
  - TASK-55.2.2
references:
  - test/fs-trace.ts
parent_task_id: TASK-55.2
priority: high
type: task
ordinal: 74390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Test 30 calls `readAccountDeclaration` directly, under a `vi.mock` trace of node:fs and node:child_process, to prove that the reader only reads, runs no git and checks the cache against the lock. A spawned binary cannot be traced in process. Same rule as R1.3.

- Move to test/reference/30-account-cache-read.test.ts, together with the two `vi.mock` calls and the trace positive control: every case that needs the trace, the in-process reader, `recordedSource`, `evaluateAccount`/`mergeAccount`/`Diagnostics`, or the in-process build trace.
- For each moved trace case whose result can be seen through `alz account which`, keep or add an acceptance case with three language-neutral proxies:
  - (a) the declaration it reports: account, source and warnings in the output;
  - (b) no writes: a snapshot of HOME, the cache, the state folder and the repository (kind, mode, content, link target), taken before and after, is equal; generalise 32's `snapshot()` into test/helpers.ts;
  - (c) no git: a `git` shim first on PATH that records its arguments to a file and exits 1 is never called. A positive control shows that `alz build` does call the shim.
- Reading inside account folders, and the exact set of files opened, stay reference-only. The PR says so and names the Rust task that covers them through alz-io's capability types (R3, TASK-55.4; R6, TASK-55.7).
- After the split, the acceptance file has no `vi.mock` and no src/ import.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 test/acceptance/30-account-cache-read.test.ts imports nothing from src/ and has no vi.mock; the R1.1 guard list drops it, apart from its use of the helpers, which R1.7 removes
- [ ] #2 The git-shim proxy has a positive control: alz build is seen calling it, and the account which cases are seen not calling it (acceptance test)
- [ ] #3 Every trace case runs under test/reference with its original name. The PR lists each one with its CLI counterpart or the reason it has none
- [ ] #4 `mise run test` passes; the case counts per project are recorded in the notes and are at least the baseline
<!-- AC:END -->
