---
id: TASK-55.2.7
title: 'R1.7: Run the acceptance suite against ALZ_BIN in one temp root per run'
status: To Do
assignee: []
created_date: '2026-11-22T17:56:00Z'
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
- [ ] #1 With ALZ_BIN unset, `mise run test` passes, the R1.1 guard list is empty, and the case counts per project match the baseline plus the counterparts added in R1.3 to R1.6 (numbers and durations before and after in the notes)
- [ ] #2 With ALZ_BIN set to an executable copy of dist/alz-standalone.mjs (`npm run build:standalone`), `mise run test:acceptance` passes; with ALZ_BIN relative or not executable, the run stops before any test with a message naming ALZ_BIN
- [ ] #3 The alz-* entries in the temp folder are the same before and after `mise run test`, for a passing run and for a run with one deliberately failing test (listings compared with comm; the command is recorded)
- [ ] #4 A spawned child that outlives its timeout fails the test within that timeout, with the argv and a process listing in the message (a reference test that spawns sleep through the helper with a 1 s timeout)
- [ ] #5 `ALZ_BIN=<absolute path of target/release/alz> npx vitest run --project acceptance test/acceptance/25-cli.test.ts` passes on the Rust skeleton, so the version assertion holds on both binaries
<!-- AC:END -->
