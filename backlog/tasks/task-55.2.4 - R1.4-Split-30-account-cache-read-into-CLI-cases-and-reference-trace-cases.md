---
id: TASK-55.2.4
title: 'R1.4: Split 30-account-cache-read into CLI cases and reference trace cases'
status: Done
assignee:
  - '@implementer'
created_date: '2026-11-22T17:55:00Z'
updated_date: '2026-11-22T22:58:00Z'
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
- [x] #1 test/acceptance/30-account-cache-read.test.ts imports nothing from src/ and has no vi.mock; the R1.1 guard list drops it, apart from its use of the helpers, which R1.7 removes
- [x] #2 The git-shim proxy has a positive control: alz build is seen calling it, and the account which cases are seen not calling it (acceptance test)
- [x] #3 Every trace case runs under test/reference with its original name. The PR lists each one with its CLI counterpart or the reason it has none
- [x] #4 `mise run test` passes; no case name is lost and the combined acceptance plus reference count is at least the baseline, with per-project counts recorded in the notes
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Implementation notes (R1.4).

Files: test/acceptance/30-account-cache-read.test.ts (rewritten: no vi.mock, no src/ import; reaches src/ only through test/helpers.ts and test/account-workspace.ts, so the R1.1 guard lists it under THROUGH_HELPERS until R1.7); test/reference/30-account-cache-read.test.ts (new: the original cases that need the trace, the in-process reader, recordedSource, evaluateAccount/mergeAccount/Diagnostics or a src scan; both vi.mock calls and the trace positive control moved with them; driven by layersInProcess); test/git-proxy.ts (new: git stand-in); test/reference/git-proxy.test.ts (new: 3 cases for the stand-in); test/helpers.ts (the task required it: snapshot(dir) added, generalised from 32's, kind/mode/content/link target; 32 still has its own copy, to be switched in R1.6).

Git stand-in: a /bin/sh script in a fresh temp dir; appends cwd and each argument to a log, then execs the real git by absolute path resolved on the caller's PATH before the stand-in's dir is put first. Behaviour of git is unchanged, so a build through it succeeds. The task text said the shim exits 1; it execs the real git instead so alz build can be the positive control (owner instruction).

Acceptance proxies (every which case runs through whichReading): (a) declaration through alz account which stdout/stderr (account, source layer, warnings, miss phrase and "fails closed"); (b) snapshot(HOME) and snapshot(workspace: cache, repo, hub) equal before and after; (c) git stand-in log empty. AC 2 positive control: "proves the git stand-in and the snapshot work": alz build is seen calling git (calls > 0, lock sha and index written) and changing the snapshot; alz account which is then seen not calling it. The same assertions hold in every which case.

Counts: acceptance 30: 80 -> 69; reference 30: 74 (+3 git-proxy). The 11 fewer in acceptance are the reference-only cases below (the trace control and the "never runs git ... opens only what it reads" case have acceptance counterparts one for one).

Reference cases (original name kept) and their CLI counterpart ("... (through alz account which)" unless noted):
Same case stays in acceptance too (pure CLI, no reader, so not moved): the 5 build/check cases (index written by build next to tree2@; written by every command; rewritten when tampered x2; directory at the index name; output and lock unchanged) are acceptance only. "warns, and does not fail the build, when the index entry cannot be written": reference keeps the original with the in-process reader; acceptance keeps the build half under the same name and ends with the which miss.
- finds a pinned path: layer through the index and reports the client layer's map: counterpart.
- treats a %s lock sha as an unreadable lock x3, misses when the lock names a commit that was never built, never reads another path's index entry and leaves it alone, misses on a malformed index entry x6, misses on a symlinked index entry, reads a path: layer without a ref in place and an edit since the build is a miss: counterparts.
- layers inside the repository are read only to warn (6 cases): counterparts (applied account plus the ignored-declaration warning text).
- proves the trace works: a build is seen running git and writing: reference only (needs the vi.mock trace); acceptance has the git stand-in positive control instead.
- never runs git, fetches or writes, and opens only what it reads: counterpart for no git, no writes and an untouched cache with decoys; the set of files opened and the read-only open flags are reference only (R3 TASK-55.4, R6 TASK-55.7: alz-io capability types).
- imports neither git nor child_process in the reader modules: reference only (scans src/ imports; no CLI view). Rust: the crate dependency graph and clippy disallowed-methods in the pure crates (R3 TASK-55.4).
- treats an edited %s as a content-hash miss x2, invalid layer.yaml in the cache as a content-hash miss, invalid layer.yaml the lock records as layer-invalid: counterparts, with the secret marker absent from the output.
- misses on a layer with a .env file, without reading it: counterpart for the miss and no marker printed; "without reading it" (no open of .env) is reference only (R3/R6).
- lock cases (no entry, no lock, symlink lock, no content hash, two entries, pinned without sha or unpinned with one, sha not belonging to the entry, git lock sha not 40 or 64 hex): counterparts.
- misses on a %s entry that is a link to a good copy x2, misses on a git entry that is missing: counterparts.
- misses on an invalid source; misses without opening anything when a layer directory is $HOME, and never touches an account folder; misses on %s that names an account folder, before any file system call on it x5; misses on a git: source with a '..' segment before probing it; layer directory gone (pinned, unpinned, local git: nowhere): counterparts that expect "fails closed" (alz account which reports source-gone here, not the reader's miss reason, because the source is no longer one the allowed entry names); no file system call or open on the account folder is reference only (R3/R6).
- misses when a layer is the folder CLAUDE_CONFIG_DIR or CODEX_HOME names; misses when the repository root lies inside an account folder: no counterpart: with CLAUDE_CONFIG_DIR set outside alz, `alz account which` keeps the value and never evaluates the layer step, so the reader's miss is not visible. Reference only; the Rust cover is the account resolver (R6 TASK-55.7).
- records a github: source as its clone URL and a local layer as its real path: no counterpart (unit of recordedSource; its effect shows in the which cases that compare the recorded source). Rust: unit test in alz-core.
- evaluateAccount agrees with mergeAccount x7: no counterpart (in-process agreement of two TypeScript functions; Rust has one function).
- lets the client layer's map win; keeps the recorded source across a ref bump; keeps the recorded source of a remote across a bump of its ref; group layer's declaration ignored; two client layers declare (acceptance: blocks, "several client layers declare an account"); repo layer at index 0; inside first layer the build refuses: counterparts. The reported sha is not shown by `which`; the bump cases prove the recorded source through an unchanged allowed entry.
- misses when manifest names another ref; two layers with the same name; cache inside the repository; more files than the limit and more bytes; does not follow a link inside a cached layer; tree2@ gone with the index still there: counterparts.

Coverage check (orchestrator, 2026-11-23): vitest list at 3a8581e vs the working tree, keyed by file and case name with the acceptance/reference folder stripped: 418 names before, 468 after, none missing (the five guard names differ only by path prefix). Run after clearing leftover temp folders: 40 files, 958 passed, 1 expected fail, 1 skipped, 58 s; acceptance 807 and reference 153 cases (882 before, 960 after). Acceptance falls by design because internal cases moved to reference. AC #4 reworded to match R1.3: per-project acceptance counts fall by design when cases move.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
30-account-cache-read is split. The acceptance file (69 cases) has no vi.mock and no src/ import: each account which case checks the reported account and warnings, an unchanged before/after snapshot of HOME and the workspace, and an empty log from a git stand-in on PATH (test/git-proxy.ts: logs cwd and NUL-separated argv, then execs the real git); a positive control shows alz build calling it. The reference file (74 cases) keeps every trace, reader, recordedSource and evaluateAccount/mergeAccount case under its original name; 11 have no CLI view and are listed with reasons. test/helpers.ts gains snapshot(dir); 32 can reuse it in R1.6.
<!-- SECTION:FINAL_SUMMARY:END -->
