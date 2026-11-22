---
id: TASK-55.2.6
title: >-
  R1.6: Move the fake-terminal, writer and source-scan cases of 24 and 32 to
  reference
status: To Do
assignee: []
created_date: '2026-11-22T17:56:00Z'
labels:
  - area:test
  - area:rust
milestone: m-7
dependencies:
  - TASK-55.2.5
references:
  - test/allow-driver.ts
parent_task_id: TASK-55.2
priority: high
type: task
ordinal: 76390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
`alz account allow` runs only for a person at a terminal, so test 32 and two cases of 24 call `run` in process with a fake `Terminal` that records prompts and gives scripted answers. A spawned binary needs a real pseudo-terminal for these cases, which is R1.8 and waits on an owner choice. Until then they move to reference, so the TypeScript code stays fully covered. Same rule as R1.3.

- To test/reference/32-account-allow.test.ts (and 24's two cases to test/reference/24-untrusted-repo.test.ts):
  - every case that passes a `Terminal` or uses `createTerminal`;
  - the 12-process `updateAllowed` lock case with test/allow-driver.ts (it moves under test/reference/);
  - the import-graph and "only terminal.ts reads standard input" scans;
  - the trace cases.
- Stays in acceptance, driven through the CLI with no terminal:
  - refusal without a terminal: a pipe, no stdin, each agent-session marker (written out, as now);
  - the usage-error cases;
  - the build and check hint cases;
  - where allowed.yaml may live, for the cases that need no answer;
  - the "real wiring" cases (R1.7 points them at ALZ_BIN instead of bundling src/cli/bin.ts).
- test/account-workspace.ts drops its `Terminal` import and parameter; the in-process variant goes to test/reference/in-process.ts.
- The PR lists every moved case. Fake-terminal cases are marked "back to acceptance in R1.8", which is the coverage the Rust oracle is missing until then.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 No file under test/acceptance, and neither test/account-workspace.ts nor test/helpers.ts, imports src/cli/terminal, src/resolve/account-allow-writer, src/core/yaml or test/import-graph
- [ ] #2 Every moved case runs under test/reference with its original name. The PR lists the fake-terminal cases as pending R1.8
- [ ] #3 `mise run test` passes; the case counts per project are recorded in the notes and are at least the baseline
<!-- AC:END -->
