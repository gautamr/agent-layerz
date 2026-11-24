---
id: TASK-55.7.17
title: >-
  R6.1b: Port the remaining terminal cases, and start the pty driver through
  test/exec.ts
status: Done
assignee:
  - '@implementer'
created_date: '2026-11-24T10:46:00Z'
updated_date: '2026-11-24T12:29:00Z'
labels:
  - area:test
  - area:rust
milestone: m-7
dependencies:
  - TASK-55.7.1
parent_task_id: TASK-55.7
priority: high
ordinal: 178390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
From R6.1 (2026-11-24): the pty driver (crates/alz-test-support pty-run, test/pty.ts) is in, and 18 terminal cases moved to test/acceptance/40-account-allow-terminal.test.ts. About 100 TypeScript-only cases in test/reference/32 that need a terminal are not yet ported: the state refusals, lock cases, the hint on build and check at a terminal, escaping, the link and state-folder cases, the one-chunk reader, and the eight-distinct-roots concurrency case (file 40 has a simplified twelve-runs-on-one-root form). Port them as acceptance cases (failing against the Rust binary until R6.11, R6.13 and R6.14, listed in test/ci/r6-pending.json with reasons, the case floor raised). Also: tests.md requires a test that runs anything but the binary under test to go through test/exec.ts with an allow-listed environment — route test/pty.ts's spawn of pty-run through test/exec.ts (or extend exec.ts with a pty mode) and give it the allow-listed environment.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Every terminal case of test/reference/32 has an acceptance twin in file 40 (or another), listed in r6-pending.json until it passes, and the case floor is raised
- [x] #2 test/pty.ts starts pty-run through test/exec.ts with an allow-listed environment, never ...process.env
- [x] #3 mise run test and the r6-pending check exit 0
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
R6.1b implementation (2026-11-24).
AC1 (partial, see 'left'): ported 112 new terminal cases (131 in files 40-42, was 18 in file 40): 40 = 16 new (prompt display with account folders, private modes and leftovers, 'yes', two answers in one chunk, subdirectory/symlink root, replace/keep/sort entries, only named targets, linked and dangling allowed.yaml, --remove x4, lock wait, foreign lock refused after the 10 s wait, link lock not followed, eight distinct roots + four runs on a shared one; 'already allowed' ending allowed); 41 = 77 (24 marker refusals at a terminal, 4 usage errors, 17 state refusals, 10 --remove refusals, relative HOME, 8 layer-dependent refusals, 11 where-allowed.yaml-may-live and link cases); 42 = 20 (hint on build/check at a terminal x9 incl. markers loop, 9 edge cases, 2 escaping). All pass against the TypeScript bundle (ALZ_BIN unset): 131/131. Against the Rust release binary 110 fail and are listed in test/ci/r6-pending.json (reason names R6.11/R6.13, and R6.14/R6.10 for hint cases); 21 pass for real (those that need no allow). Case floor raised 1101 -> 1214.
AC2: test/exec.ts gained spawnInteractive (own process group, timeout with the process-group listing, env = PATH + given + TMPDIR, never process.env); test/pty.ts starts pty-run through it. Shared set-up in test/allow-terminal.ts.
AC3: mise run lint ok; mise run test 56 files, 1309 passed, 288 expected fail; ALZ_BIN=target/release/alz acceptance + check-r6-pending.ts: '1214 cases (floor 1214), 356 failing, 356 listed as R6 pending', exit 0.
Deviations, on purpose: (a) display control-character check asserts stdout only, because on a terminal the TS line editor writes cursor controls to stderr; (b) 'no hint for a repository that is HOME / inside CLAUDE_CONFIG_DIR' no longer asserts the exit status: spec 6 stops a command in a directory that touches an account folder (Rust does, TS exits 0); (c) check-tamper case appends to the generated file instead of overwriting it, since an overwritten file loses the header and Rust reports a collision; (d) each terminal case has a 60 s timeout (vi.setConfig, TERMINAL_CASE_MS) with 50 s pty-run deadlines, as the machine is often loaded.
Left (TS-only, no acceptance twin possible): the fs-trace cases (no write outside the state dir, no subprocess, no open in an account folder), the import-graph and stdin source scans, createTerminal in-process cases, the updateAllowed uid-unknown case and the in-process writer lock input; the lock cases are covered through the binary instead. The reference copies are all kept. Not done: the concurrency case does not assert that some process met a held lock (the reference read that from its driver); only the end state (9 sorted unique entries, no leftovers).

Correction of the counts above: 113 new cases (131 in files 40-42, was 18); 109 new entries in r6-pending.json (127 of the 131 fail against the Rust binary, 18 of them listed before); 4 pass for real against Rust (the hint cases that need no allow: no hint without a declared account, no hint inside CLAUDE_CONFIG_DIR or at HOME, check's own comparison failing).

Follow-up: the two hint cases (repository inside the CLAUDE_CONFIG_DIR folder, repository that is HOME) assert the spec's behaviour again, exit 1 with an error diagnostic and no hint (spec 6 and 10.1), as referenceDrift('TASK-55.7', ...): expected failures against the TypeScript bundle, passing against the Rust binary. Deviation (b) above is withdrawn. Counts unchanged: 1214 cases, floor 1214, 356 failing and listed.

Verified by the lead (2026-11-24): merged (129cc58); 113 terminal cases ported (files 40-42, 131 in all, all pass against the reference); 127 listed in r6-pending; the two HOME / CLAUDE_CONFIG_DIR hint cases assert the spec's exit status as referenceDrift('TASK-55.7'). The reference cases left in test/reference/32 are in-process checks with no binary twin (fs traces, import and stdin scans, the in-process terminal); their Rust equivalents are crates/alz-io/tests/standard_input_guard.rs, the no-open-inside-account-folders tests and the stand-in git that records no call. Floor 1216 (1101 + 2 from R6.15 + 113).
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
The remaining terminal cases run through the pty driver as acceptance cases in files 40 to 42 (131), the driver starts through test/exec.ts with an allow-listed environment, and terminal cases have a 60 s timeout; failing ones are listed in r6-pending until R6.10, R6.13 and R6.14.
<!-- SECTION:FINAL_SUMMARY:END -->
