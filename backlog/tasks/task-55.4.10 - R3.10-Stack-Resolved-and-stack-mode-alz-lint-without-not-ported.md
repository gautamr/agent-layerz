---
id: TASK-55.4.10
title: 'R3.10: Stack<Resolved> and stack-mode alz lint, without [not-ported]'
status: Done
assignee:
  - '@implementer'
created_date: '2026-11-23T07:42:00Z'
updated_date: '2026-11-23T09:50:00Z'
labels:
  - area:rust
  - area:resolve
milestone: m-7
dependencies:
  - TASK-55.4.9
parent_task_id: TASK-55.4
priority: high
type: task
ordinal: 111390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Plan for this step. Removes R2's `[not-ported]`, and fixes TASK-71 end to end through lint.
- alz-io `resolve/working_tree.rs`: an unpinned `path:` opens its `Dir` through the walk (TASK-52 refusal), scans it with `Purpose::Build`, and is not checked against the lock.
- alz-io `resolve.rs`: `stack(&Manifest, Option<&Lock>, &EnvSnapshot, &GitRunner, &CacheDir) -> Result<Stack<Resolved>, Diagnostics>`, for each entry in stack order:
  1. reads `layer.yaml` as a regular file through `Dir` (O_NOFOLLOW, 1 MiB);
  2. parses it with R2's `LayerMeta`;
  3. records `Pin { rev, sha }`.
  Then §6 step 2 in alz-core's `stack/validate.rs`: visibility order and duplicate names, with behaviour from src/resolve/layer-load.ts validateStack.
- alz `command/lint.rs`, stack mode:
  1. builds `EnvSnapshot`, `CacheLocation`, `CacheDir` and `GitRunner` only when a `git:`, `github:` or pinned `path:` entry needs them;
  2. resolves;
  3. lints each layer's meta;
  4. prints `lint ok: <names>` or `lint failed: …`, as in R2's layer mode (§10.1).
  `Code::NotPorted` and its message are removed.
- A `git:` source holding credentials stops at the manifest, before any cache or git access (R2.6's rule).
- assert_cmd tests in `crates/alz/tests/lint.rs`.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 alz lint in a repository resolves every stack entry, validates the stack (visibility order, duplicate names, layer.yaml a regular file) and prints lint ok: <names>; no output holds not-ported (assert_cmd tests)
- [x] #2 alz lint with a git:https://user:token@host/x.git@v1 source exits 1 with error [source] and the §5.2 message, runs no git, creates no cache directory, and stderr holds no token (assert_cmd test; TASK-71 AC #1 and AC #2 through lint)
- [x] #3 An unpinned path: layer is read in place and never checked against the lock, while a cached git: or pinned layer is (assert_cmd tests)
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
From R3.8 (2026-11-23): wire the origin refusals (OriginRefusal::into_diagnostic: account folder, '..' in a local git path, invalid file:// URL) into stack resolution.

Implementer (2026-11-23), R3.10 done, uncommitted on feat/rust-r3-task-55.4.
Design: alz-core stack/resolved.rs (Pin, ResolvedLayer, ResolvedStack, ResolvedEntry/FailedEntry, StackReport::of: pure) and stack/validate.rs (§6 step 2: duplicate-layer, visibility, with the reference's pairwise rule); lint.rs gets read_stack (manifest+lock together) and read_layer (LayerMeta from a stack entry); new codes duplicate-layer and visibility; Code::NotPorted removed. alz-io resolve/stack.rs StackResolver (account folders from the EnvSnapshot, with real paths via with_real_home/with_real_configured; cache and GitRunner (Deadlines::STANDARD) made on first use only, and a failure told once; unpinned path: via Dir::open + scan(Build), pinned path: via PinnedTreeReader after the walk accepted the dir, git:/github: via locate_git_source/github_origin + Fetcher; OriginRefusal::into_diagnostic wired; layer.yaml read through Dir (regular file, 1 MiB) then LayerMeta; .env files are env-file errors). Binary: command/lint.rs stack mode.
Spec: §6 step 2 now states duplicate-layer/visibility/env-file and the per-entry reporting; §10.1 not-ported sentence replaced by the stack-mode paragraph (names = one per entry, entry as written when unresolved; no summary when manifest/lock invalid).
AC #1 evidence: crates/alz/tests/stack_lint.rs (17 tests): path stack prints 'lint ok: personal, repo' with no cache made; several errors reported together in stack order with summary 'lint failed: 4 errors in ...'; duplicate-layer; link/missing layer.yaml; git: file:// and local path; github: through insteadOf+fake ssh; failed fetch; refusals (account folder, link into one, $HOME, git: in an account folder, '..' local path, file:// without path). crates/alz/tests/lint.rs: not-ported tests replaced by real outcomes. alz-core stack::resolved tests (8). No 'not-ported' remains in crates, test or docs (grep).
AC #2 evidence: stack_lint a_source_with_credentials_is_refused_before_git_or_the_cache...: four URLs; exit 1, stderr exactly the §5.2 error [source] line, token absent, a spy git on PATH never ran, no xdg cache dir made; a_user_name_without_a_password... for ssh://git@ and git@host:o/x.git. (The refusal happens in the manifest parse, R2.6; lint stops before the resolver.)
AC #3 evidence: an_unpinned_path_layer_is_read_in_place_and_never_checked_against_the_lock (lock with a wrong hash, then an edit: still ok); a_cached_git_layer_is_checked_against_the_lock... (matching lock read with the source removed; wrong hash -> error [cache] no longer matches); a_pinned_path_layer_is_read_from_the_cache_and_checked_against_the_lock.
Acceptance (release binary): 35/29/34 files: 79 cases, 69 pass, 10 fail; the 10 are alz build / init / account which cases (35: 2 build drift; 29: 1 build drift + 5 build + 1 init + 1 build twin). test/acceptance/35: 'alz lint gets past a ref that is text' now asserts the real outcome (exit 1, error [source] cannot resolve ref 1.10 for 'path:./.layer', since the fixture is no git repo); the reference gives the same line.
Checks: mise run rust:ci green (fmt, clippy -D warnings, tests, doc, msrv, deny, audit; one run had git_runner a_call_past_its_deadline fail once under load, passes alone 3 of 3, unrelated). mise run lint green.
Decisions to confirm: (1) ResolvedStack is a plain type, not Stack<Resolved> typestate; (2) unset or relative HOME leaves '/' as home for the account folders (the user-database fallback is TASK-72); (3) unspecified messages: layer_unreadable for NameNotSafe/Unreadable of a working tree; (4) summary naming rule above.

Implementer: mise run test (TypeScript reference + acceptance against it) took about 800 s on a loaded machine and 7 files had one timeout-style failure each (26, 28, 30, 31 acceptance; 30, 32, run-root, git-proxy reference); rerun of those 9 files alone: all pass (157 passed + 8 expected drift failures in 28/26/35/git-proxy; 440 passed in the other five). No src/ change.

Verified by the lead (2026-11-23) on an idle machine: mise run rust:ci exits 0 (624 tests; stack_lint 17), lint passes, mise run test 48 files, 1118 passed, 44 expected fail, 1 skipped in 75 s (the implementer's timeouts were machine load), src/ unchanged, no not-ported left outside a test asserting its absence. Lead accepts: the summary names one entry per stack entry (layer name, or the entry as written when unresolved or failing the schema; no summary when the manifest or lock is invalid); the messages for an unreadable working tree and a missing git executable; visibility located at the entry source. For the R3 review: ResolvedStack is a plain validated type, not the planned typestate; an unset or relative HOME leaves / as home for the account folders until TASK-72 (R6).
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
alz lint in a repository now resolves the whole stack: unpinned and pinned path: layers, git: and github: sources through the cache and the lock, with account-folder, '..', file:// and credential refusals, then lints every layer and the stack rules (duplicate names, visibility order), reports every diagnostic in stack order and prints the summary line. The temporary not-ported error is gone from code, tests and spec.
<!-- SECTION:FINAL_SUMMARY:END -->
