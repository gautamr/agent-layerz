---
id: TASK-55.2.5
title: >-
  R1.5: Split 31-account-which into CLI cases and reference resolver and trace
  cases
status: To Do
assignee: []
created_date: '2026-11-22T17:55:00Z'
labels:
  - area:test
  - area:rust
milestone: m-7
dependencies:
  - TASK-55.2.4
parent_task_id: TASK-55.2
priority: high
type: task
ordinal: 75390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Test 31 calls `resolveAccounts` directly 13 times, some with inputs the CLI cannot give yet (`run:` from the paused alz run, TASK-50.3). It also proves under the fs trace that `alz account which` opens nothing outside a short list and runs no git, proves through source-graph scans (`valueImports`) that account resolution imports no git, cache, pipeline or allowed.yaml writer, and checks O_NONBLOCK on `readRegularFileWithId`. Same rule as R1.3; uses the proxies and the git shim from R1.4.

- To test/reference/31-account-which.test.ts: the resolver-input cases with no CLI form, the trace cases, the import-graph cases, the `parseYamlLimited` alias control and the `readRegularFileWithId` unit.
- CLI counterparts:
  - resolver cases that the CLI can reach become `alz account which` cases (`--use`, accounts.yaml, allowed.yaml, the stack);
  - "opens nothing outside the list and runs no git" becomes the R1.4 snapshot and git-shim proxies;
  - O_NONBLOCK becomes a FIFO put at the accounts.yaml path and at the allowed.yaml path: `alz account which` returns well inside the spawn timeout and reports the file as unreadable;
  - the alias limit is seen through the CLI with the payloads the file already uses.
- The usage cases (lines 258-270) and the control-character case stay in acceptance. R1.7 rewrites their assertions in the decision-26 form.
- The 1,959-line file may be split into two acceptance files by topic, if the review is easier that way.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 test/acceptance/31-account-which.test.ts (and any file split from it) imports nothing from src/ and has no vi.mock
- [ ] #2 A FIFO at the accounts.yaml path, and another at the allowed.yaml path, make alz account which finish and report the file as unreadable, without waiting for a writer (acceptance test)
- [ ] #3 Every moved case runs under test/reference with its original name. The PR lists each one with its CLI counterpart or the reason it has none (no CLI form yet; the Rust task that tests it)
- [ ] #4 `mise run test` passes; the case counts per project are recorded in the notes and are at least the baseline
<!-- AC:END -->
