---
id: TASK-55.2.7
title: 'R1.7: Run the acceptance suite against ALZ_BIN in one temp root per run'
status: Done
assignee:
  - '@implementer'
created_date: '2026-11-22T17:56:00Z'
updated_date: '2026-11-23T03:02:00Z'
labels:
  - area:test
  - area:rust
milestone: m-7
dependencies:
  - TASK-55.2.3
  - TASK-55.2.4
  - TASK-55.2.5
  - TASK-55.2.6
references:
  - TASK-54
  - >-
    backlog/decisions/decision-26 -
    Spec-owns-streams-diagnostics-and-exit-codes-clap-owns-help-usage-errors-and-version-sanitised-by-alz-owner-TASK-55.1.2.md
parent_task_id: TASK-55.2
priority: high
type: task
ordinal: 77390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The switch from in-process runs to spawning a binary. It includes TASK-54 (one temp root per run, removed at the end) and that task's AC #2: fail fast with a clear message.

Global setup for the acceptance project (test/setup/):
- One run root, `mkdtemp(<os temp>/alz-suite-)`, holding `bin/`, `tmp/` and a `pid` file. Teardown removes it whether tests pass or fail.
- Setup also removes stale `alz-suite-*` roots, and nothing else. It does so only when all of these hold: the entry is a real directory (lstat, not a link), its owner is the current uid, its real path is directly inside the temp folder, and its `pid` file names a process that no longer exists.
- Every temp folder the tests make lives under `<root>/tmp`: a helper `tempDir(prefix)` replaces the direct `mkdtempSync(join(tmpdir(), ...))` calls in 26, 27 and 32, and every child (alz, git, sh) gets `TMPDIR=<root>/tmp`.
- ALZ_BIN, when set, must be an absolute path to an executable regular file, checked after realpath. Anything else stops the run before the first test, with a message naming ALZ_BIN. It is never run through a shell.
- When ALZ_BIN is unset, the TypeScript reference is built from src/cli/bin.ts with esbuild, using the `build:standalone` options, into `<root>/bin/alz`, mode 0755. Its first line is `#!<absolute node path>`, so cases that set PATH (26 line 844, `PATH=/nonexistent`) still start it.

Spawning:
- `layers()` and `alz()` spawn the binary asynchronously: an argument array, the given cwd, and exactly the env {HOME, PATH, ALZ_CLAUDE_BIN, TMPDIR, ...extra}. stdin is ignored; stdout and stderr are captured as UTF-8; the result is {code, stdout, stderr}.
- Each child starts in its own process group with a timeout (15 s by default, below the 20 s test timeout; a caller can raise it). On expiry the helper records `ps -o pid,ppid,stat,etime,command` for the group, kills the group, and fails with the argv, the elapsed time, stderr and that listing. A child that ends on a signal fails clearly.
- The tests' own git and sh calls go through one helper with the same timeout and dump.

Assertions in the decision-26 form (its consequence for R1):
- 25 and 27 `--version`: exit 0, empty stderr, stdout contains the package.json version.
- 27 `--help`: exit 0, stdout not empty.
- Usage errors in 31 and 32: exit 2, empty stdout, stderr not empty.
- 24 lines 589 and 599 (`not.toContain("TypeError")`): exit code 1, not 70, and stderr holds only diagnostic lines.
- The control-character case in 31 is unchanged.
- 32's "real wiring" cases spawn ALZ_BIN.

Other changes:
- mise.toml gains `test:acceptance` (`vitest run --project acceptance`, honouring ALZ_BIN).
- The R1.1 guard list becomes empty, and the guard also rejects `tmpdir()` in acceptance files.
- Out of scope: CI changes (pointing a Rust CI job at target/release/alz is R2 or later, with the owner's approval), and bypassing the xcrun git shim (see TASK-54).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 With ALZ_BIN unset, `mise run test` passes, the R1.1 guard list is empty, and the case counts per project match the baseline plus the counterparts added in R1.3 to R1.6 (numbers and durations before and after in the notes)
- [x] #2 With ALZ_BIN set to an executable copy of dist/alz-standalone.mjs (`npm run build:standalone`), `mise run test:acceptance` passes; with ALZ_BIN relative or not executable, the run stops before any test with a message naming ALZ_BIN
- [x] #3 The alz-* entries in the temp folder are the same before and after `mise run test`, for a passing run and for a run with one deliberately failing test (listings compared with comm; the command is recorded)
- [x] #4 A spawned child that outlives its timeout fails the test within that timeout, with the argv and a process listing in the message (a reference test that spawns sleep through the helper with a 1 s timeout)
- [x] #5 `ALZ_BIN=<absolute path of target/release/alz> npx vitest run --project acceptance test/acceptance/25-cli.test.ts` passes on the Rust skeleton, so the version assertion holds on both binaries
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
2026-11-23 (implementer). Done, not committed. Files: new test/setup/{run-root,alz-bin,stale-roots,remove-tree}.ts (one root-level globalSetup, registered in vitest.config.ts, serves both projects: run root alz-suite-XXXX with bin/, tmp/, pid; teardown removes it pass or fail; stale sweep by lstat+uid+realpath+dead pid), test/run-root.ts (tempDir, tmpRoot, alzBin via inject), test/exec.ts (spawnCaptured async, capture/tool sync; own process group, 15 s default, ps listing, group kill, signal ends fail), test/reference/{exec,run-root,account-in-process}.ts(.test.ts). layers() now spawns ALZ_BIN with env exactly HOME, PATH, ALZ_CLAUDE_BIN, TMPDIR, ...extra. All tmpdir()/mkdtemp calls in test/ now use tempDir; every git/sh/mkfifo/rm call in acceptance goes through tool/capture. Guard list is empty and a second guard rejects tmpdir in acceptance files. mise.toml: test:acceptance. Assertions in decision-26 form in 25, 27, 31, 32, 24 (24: stderr lines must all match the diagnostic format and exit code in {0,1}; the first case already allowed 0, so 'exit 1' was read as 'handled, not 70'). 32 real wiring now spawns the binary under test. 26 PATH-less case now uses a PATH with a node script and no git, so a #!/usr/bin/env node copy of dist/alz-standalone.mjs starts too. Reference 31 'never runs git' case: workspace roots are now real paths (a spawned binary reports its real cwd), so the case starts the command from the link-form name (/var vs /private/var) to keep 'working directory resolved once'.
AC1: mise run test with ALZ_BIN unset: 44 files, 1057 passed, 1 expected fail, 1 skipped (1059), 66 s. Baseline HEAD (R1.6, lead-verified): 42 files, 1028 passed + 1 xfail + 1 skip (1030), 73 s. Per project: acceptance 701 passed + 1 skipped (702, unchanged from R1.6), reference 356 passed + 1 expected fail (357; was 327+1) = +29 new cases (exec, run-root, stale roots, ALZ_BIN, guard). Guard lists: DIRECT and THROUGH_HELPERS empty.
AC2: ALZ_BIN=<chmod 755 copy of dist/alz-standalone.mjs> mise run test:acceptance: 31 files, 701 passed, 1 skipped, 39 s. ALZ_BIN relative: exit 1 before any test, message 'ALZ_BIN must be an absolute path to an executable file, but ALZ_BIN="scratchpad/alz" is not an absolute path'; mode 644 copy: same message ending 'is not executable'. (vitest also prints 'No test files found' beneath the error; the setup error is the first line.)
AC3: command: ls "$TMPDIR" | grep '^alz-' | sort > before.txt; mise run test; ls ... > after.txt; comm -3 before.txt after.txt | wc -l. Passing run: 0 differing lines. Run with a deliberately failing test in each project (reverted): 2 failed, 0 differing lines. Before the change one run left about 2289 folders.
AC4: test/reference/exec.test.ts: spawnCaptured('sleep', ['30'], timeoutMs 1000) rejects in under 5 s with the argv and the ps listing (pid,ppid,pgid,stat,etime,command); a grandchild 'sleep 31' is killed with the group; capture() times out the same way; a child that kills itself with SIGKILL fails with 'ended on signal'.
AC5: cargo build --release --locked --package alz; ALZ_BIN=$PWD/target/release/alz npx vitest run --project acceptance test/acceptance/25-cli.test.ts: 1 passed (stdout 'alz 0.1.0', empty stderr). No crates/ change.
TASK-54 evidence: once, in a run of 26 against the dist copy with an empty alz-* temp folder (so not H1 alone), the tests' own sync 'git config --local user.name' took 132814 ms although the timeout is 15 s; the sync spawnSync returned only after about 132 s (the child apparently did not die on SIGKILL for that long). The test failed with the timeout message, but I had filtered away the ps listing, so it was lost; a rerun passed (83/83). Open: the sync helper blocks the worker, so the 20 s vitest timeout cannot fire there and the listing is taken after the kill. An async tool() would give a listing before the kill.

Verified by the lead (2026-11-23): `mise run test` with ALZ_BIN unset: 44 files, 1057 passed, 1 expected fail, 1 skipped (1059), 107 s, while a stop-hook suite ran alongside; `mise run lint` passes; guard WRITTEN list is [] and no acceptance file names src/. AC #2: ALZ_BIN=<scratch>/alz-ts (chmod 755 copy of dist/alz-standalone.mjs) `mise run test:acceptance`: 31 files, 701 passed, 1 skipped, 38 s; mode 644 copy and relative target/release/alz each stop with 'ALZ_BIN must be an absolute path to an executable file', exit 1. AC #3: comm -3 of the alz-* listings showed one alz-suite-* entry after a run; ps showed it belonged to a concurrent `npx vitest run` started by the verify-on-stop hook (.layer/hooks/hooks.yaml), and it was gone within 1 s when that run ended, so this run's own root was removed. AC #5: ALZ_BIN=$PWD/target/release/alz npx vitest run --project acceptance test/acceptance/25-cli.test.ts: 1 passed. Deviation from the description, accepted: the two former not.toContain('TypeError') cases in 24 assert exit 0 or 1 (never 70) with only diagnostic lines on stderr, because the reference exits 0 with a warning on one of them.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
The acceptance project now spawns the binary under test (ALZ_BIN, or the TypeScript reference bundled into the run root) with an exact environment, a process group and a 15 s timeout that fails with argv and a ps listing. One global setup gives both projects a single alz-suite-* run root, removed at teardown, and sweeps only stale roots of dead runs owned by the user; every test temp folder lives under it, so a run no longer leaves alz-* folders behind (about 2289 before). The guard list of acceptance files that reach src/ is empty, a second guard rejects tmpdir in acceptance files, and assertions follow decision-26. The Rust skeleton passes the --version case. Adds mise run test:acceptance.
<!-- SECTION:FINAL_SUMMARY:END -->
