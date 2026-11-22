---
id: TASK-55.2.5
title: >-
  R1.5: Split 31-account-which into CLI cases and reference resolver and trace
  cases
status: Done
assignee:
  - '@implementer'
created_date: '2026-11-22T17:55:00Z'
updated_date: '2026-11-22T23:07:00Z'
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
- [x] #1 test/acceptance/31-account-which.test.ts (and any file split from it) imports nothing from src/ and has no vi.mock
- [x] #2 A FIFO at the accounts.yaml path, and another at the allowed.yaml path, make alz account which finish and report the file as unreadable, without waiting for a writer (acceptance test)
- [x] #3 Every moved case runs under test/reference with its original name. The PR lists each one with its CLI counterpart or the reason it has none (no CLI form yet; the Rust task that tests it)
- [x] #4 `mise run test` passes; no case name is lost and the combined acceptance plus reference count is at least the baseline, with per-project counts recorded in the notes
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Implementation notes (R1.5).

Files: test/acceptance/31-account-which.test.ts (rewritten: no vi.mock, no src/ import, no trace; reaches src/ only through test/helpers.ts and test/account-workspace.ts, so the R1.1 guard lists it under THROUGH_HELPERS until R1.7; moved from DIRECT in test/guards/acceptance-imports.test.ts); test/reference/31-account-which.test.ts (new: 23 cases, both vi.mock calls, original names and describe nesting kept); test/helpers.ts (snapshot(dir) now records an unreadable file as `<unreadable>` instead of dropping the whole folder; the mode 000 case of the walk needed it).

Acceptance proxies: every `alz account which` case goes through `which`, which takes snapshot(HOME) and snapshot(workspace) before and after (must be equal) and runs alz with the R1.4 git stand-in first on PATH (its log must be empty). This makes the whole file, not only the moved cases, check "writes nothing and runs no git". Positive control: "proves the proxies work: a build is seen running git and writing (through the git stand-in)".

decision-26: the usage cases (exit 2 with "usage: alz <command>", unknown subcommand, "stops `alz account which --use` with a bad name") and the control-character-removal case for unknown commands stay in acceptance unchanged; R1.7 rewrites their assertions in the decision-26 form. The quoting describe (control characters in repository text, which alz owns, class 1) stays as is.

Reference cases (original name kept), with the CLI counterpart ("... (through alz account which)" unless noted) or the reason there is none:
- lets alz run's explicit choice beat alz use's, in the resolver: no CLI form yet (`run:` comes from the paused alz run, TASK-50.3). Rust: unit test of the account resolver (R6, TASK-55.7).
- fails closed for a bad alz run or alz use name, building no path and never falling to a lower step: the `--use` half has acceptance "stops `alz account which --use` with a bad name before any lower step, via the usage check" (exit 2, kept); the `run:` half has no CLI form yet (TASK-50.3). Rust: R6 unit test.
- ignores a file the user does not own, in the resolver: no CLI form (needs a second uid). Rust: R6 unit test with an injected uid.
- ignores both files when the platform has no uid to compare with: no CLI form (needs a platform without uids). Rust: R6.
- ignores a file whose path names an account folder, before any file system call on it: counterpart with the same name + suffix (both files located by XDG variables, reported as ignored: it lies inside an account folder); "before any file system call" is reference only (R3 TASK-55.4 / R6: alz-io capability types).
- never reads the values of __MISE_DIFF, DIRENV_DIFF or an ALZ_USE_ variable, only the listed names: no new counterpart, the existing acceptance cases "never puts the value of the variable in any output" and "ignores a forged ALZ_USE_CLAUDE in the environment" show the visible part; the names read are reference only (needs the env proxy). Rust: the resolver takes an environment snapshot value (R6).
- proves the alias payload parses within the limit and is refused only past it: counterpart (50 aliases give "layer.yaml is not valid", 150 give "layer.yaml cannot be read", through the same walk; asserts 50 <= 100 < 150 against the spec constant). The accounts.yaml and allowed.yaml alias payloads were already acceptance cases.
- opens a regular file without waiting for a writer (O_NONBLOCK), so a swapped-in FIFO cannot hang it: unit of readRegularFileWithId; counterparts are the new FIFO cases below (accounts.yaml path, allowed.yaml path) and the existing FIFO layer.yaml case. The "open flags include O_NONBLOCK" half is reference only. Rust: alz-io test (R3 TASK-55.4).
- gives the default account and opens no lock or cache file for a root that has no entry: counterpart (the default account, "this repository is not allowed", cache deleted so reading the stack would show; unchanged snapshot, no git). The set of files opened is reference only (R3/R6).
- applies the named account for a matching entry: counterpart is the existing acceptance case "takes the layer's account once allowed.yaml holds a matching entry" (stderr empty there too); the in-process `warnings == []` form is reference only.
- proves the trace works: a build is seen running git and writing: counterpart "proves the proxies work: a build is seen running git and writing (through the git stand-in)".
- never runs git, writes or opens anything it need not, in an allowed repository: counterpart (reported claude/codex accounts, stderr empty, planted account folder content not printed; unchanged snapshot, no git). The read-only open flags, the allowed set of opened and listed paths, the single listing of $HOME and single realpath of the cwd and no touch of the planted folder are reference only (R3 TASK-55.4, R6 TASK-55.7).
- opens no stack file for a repository that is not allowed, and runs no git: counterpart (cache deleted, still the default account and "this repository is not allowed"; no git, unchanged snapshot).
- proves the import graph sees a git import: the build command has one; does not import git, the tree cache, source fetching, the build pipeline or child_process; does not import the writer of allowed.yaml, or the build and check hint (TASK-50.11): no CLI view (source-graph scans of src/). Rust: the crate dependency graph and clippy disallowed-methods in the pure crates (R3 TASK-55.4); the writer's absence from `which` is also covered by the unchanged-snapshot proxy.
- opens nothing in the configured folder: only allowed.yaml is read (CLAUDE_CONFIG_DIR names the repository): no new counterpart; the behaviour is in the existing "fails closed on the blocked folder for an allowed repository when the variable names ..." cases; the open set is reference only (R3/R6).
- reads nothing from the file system in the resolver (relative HOME): the existing acceptance case "ignores both files with one warning, sets no folder and fails closed for a named account" now also checks the unchanged snapshot and empty git log; "no file system call" is reference only.
- makes no file system call under an account folder, and skips the user config step; treats a link into an account folder as an account folder, and hands the file system only the link's own path: counterparts (repository: none, user config step skipped, the planted credentials file is not printed; unchanged snapshot). The file system calls are reference only.
- covers the two files located by XDG variables: counterpart is the "ignores a file whose path names an account folder" case above.
- covers a folders key, an allowed root and a stack source: counterpart (account-config-folder and account-allowed-entry warnings, and the layer fails closed on the cache miss); no file system call is reference only.
- covers a platform where the owner cannot be read: no file of either folder is touched: no CLI form (no uid); reference only. Rust: R6.

New acceptance cases with no reference original: FIFO at the accounts.yaml path and FIFO at the allowed.yaml path (AC 2): `alz account which` finishes, reports "(ignored: it is not a regular file of at most 1 MiB that can be read)" and applies the default account; each also asserts it ended within 10 s. Limit: the suite runs alz in this process, so a blocking open would block the vitest worker and could not time out; R1.7 spawns the binary, which makes the time limit real. The FIFO is mode 0600 so the safety checks pass and the read is what is tested.

Counts: acceptance 807 -> 795 (31: 128 cases before, 116 after: 23 moved, 11 added); reference 153 -> 176 (+23); both include the 1 skipped and 1 expected fail. Total 960 -> 971. Name coverage: vitest list of the original file against the union of the new acceptance and reference files: 96 unique names before, none missing, 11 added (the counterparts and FIFO cases above).

Orchestrator rerun after clearing $TMPDIR/alz-*: 41 files, 969 passed, 1 expected fail, 1 skipped, 57 s; lint clean; grep: no vi.mock or src import in acceptance/31. AC #4 reworded as in R1.3 and R1.4. Note: the FIFO cases' 10 s bound only becomes real when R1.7 spawns the binary.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
31-account-which is split. Every alz account which case in acceptance now also checks unchanged snapshots of HOME and the workspace and an empty git stand-in log; 11 CLI counterparts and two FIFO cases (accounts.yaml, allowed.yaml) were added. 23 cases moved to test/reference under their original names (open-flag and file-set traces, import-graph scans, readRegularFileWithId, env-read proxies, cases with no CLI form until alz run), each listed with its Rust task. snapshot() records unreadable files instead of dropping the folder. Usage, unknown-subcommand and control-character assertions stay for R1.7 (decision-26).
<!-- SECTION:FINAL_SUMMARY:END -->
