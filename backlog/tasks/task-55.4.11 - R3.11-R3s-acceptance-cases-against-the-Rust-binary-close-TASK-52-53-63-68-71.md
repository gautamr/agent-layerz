---
id: TASK-55.4.11
title: >-
  R3.11: R3's acceptance cases against the Rust binary; close TASK-52, 53, 63,
  68-71
status: In Progress
assignee:
  - '@implementer'
created_date: '2026-11-23T07:42:00Z'
updated_date: '2026-11-23T09:50:00Z'
labels:
  - area:rust
  - area:resolve
milestone: m-7
dependencies:
  - TASK-55.4.10
parent_task_id: TASK-55.4
priority: high
type: task
ordinal: 112390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Plan for this step (TASK-55.4, "Acceptance cases R3 must pass against ALZ_BIN"). It changes the suite only, never src/.
- Add `writeLock(ws, entries)` to test/helpers.ts: a version 1 lock with the content hash from test/spec/content-hash.ts, so the lock cases need no build.
- Lint twins, the same fixture and verdict with `alz lint` in the repository:
  - 16: resolves `path:../hub/layer` at layer-v0.1.0; hub commits after the tag change nothing; a ref the hub lacks is `error [source]`;
  - 24: a lock sha that is not a commit id; a manifest ref read as an option;
  - 28: every case of "git cache keys", "cached content is checked against the lock", "pinned path: layers are checked against git", "pinned layer trees", "layer.yaml" and "an untrusted lock", and "lint refuses a tampered cached layer" on a written lock;
  - 30: the cache index entry and XDG_CACHE_HOME empty or relative.
- New cases, `referenceDrift` where the reference fails, each naming its task:
  - TASK-52: a layer at the fake HOME, reached through a link into `~/.claude-x`, and a cache root in an account folder;
  - TASK-53: an empty and a relative XDG_CACHE_HOME, with the repository tree compared before and after;
  - TASK-63: a pre-commit hook running `alz lint` in a main checkout and in a linked worktree leaves the index and repository byte-identical;
  - TASK-68: test 26's remote failure with the whole message, and a fake ssh writing ESC, CR and U+202E;
  - TASK-69: the byte 0xFF, Linux only;
  - TASK-71: credentials.
- Run: `mise run rust:build`, then `ALZ_BIN=$PWD/target/release/alz mise run test:acceptance` on these files and cases, and `mise run test` with ALZ_BIN unset.
- Close TASK-52, TASK-53, TASK-63, TASK-68, TASK-69, TASK-70 and TASK-71: check each criterion with this run's evidence (their criteria name the lint twins, owner answer 7; the build and check forms are written into the suite where missing and checked again in R5, TASK-55.6), and move them to In Review with the pull request.
- Record in TASK-55.4's notes the list of cases and the Rust test behind each case of 16, 24, 28 and 30 (AC #1).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Every case listed in the description passes against ALZ_BIN=target/release/alz; the list with counts, and the Rust test behind each case of 16, 24, 28 and 30, is in TASK-55.4's notes
- [ ] #2 mise run test passes with the reference, and each new drift case is an expected failure there that names its task
- [ ] #3 TASK-52, TASK-53, TASK-63, TASK-68, TASK-69, TASK-70 and TASK-71 have every criterion checked with this run's evidence and go to In Review with this pull request
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
From R3.4 (2026-11-23): add the exact texts of the cache-integrity message, 'layer directory not found' and 'cannot resolve ref' to the spec (§6 / §10.1), matching crates/alz-core/src/stack/resolution_failure.rs.

From R3.3 (2026-11-23): run crates/alz/tests/git_runner.rs against git 2.30 if a 2.30 build can be installed (e.g. through mise); otherwise record that the floor's argv is verified only on the git found (2.50.1).

From R3.6 (2026-11-23): add to the spec the messages 'cannot use the layer cache at <dir>', "layer '<s>' is not a directory" and "layer '<s>' cannot be opened" (alz-io fs/walk_error.rs, cache_error.rs).

From R3.7 (2026-11-23): the spec needs (a) the rule for a name that is valid Unicode but not a SafeRelPath (a backslash, .GIT, .git.) — implemented as NameNotSafe, a layer-invalid miss right after NameNotUtf8 — and its build message, and (b) the build message for an unreadable file or folder in a layer.

From R3.8 (2026-11-23): add a Linux-only case for a fetched git clone that holds a file name that is not valid UTF-8 (§5.3).
<!-- SECTION:NOTES:END -->
