---
id: TASK-55.2.3
title: 'R1.3: Move the TypeScript-internal cases of 21, 24, 26, 28 and 29 to reference'
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
parent_task_id: TASK-55.2
priority: high
type: task
ordinal: 73390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Rule for this subtask and R1.4 to R1.6: a case that calls a src/ function moves to test/reference/NN-<same name>.test.ts with its name, body and assertions unchanged. If the behaviour can be seen through the CLI today, an acceptance case is added that shows it through the CLI; the PR lists each moved case with its CLI counterpart, or with "not reachable by the CLI yet" and the Rust task that will test it. Coverage of the TypeScript code is never lost: the moved case still runs.

Cases:
- 21: `parseStackOption` table. Counterpart: `alz init --stack` with a bare path, `path:...@ref` and `github:...@v` writes those sources and refs into layer.yaml.
- 24: the two `resolveAccounts` cases ("a refused account declaration, at resolution"). Counterpart: `alz account which` shows the ignored declaration and its one `account-declaration-ignored` warning. The two fake-terminal cases stay for R1.6.
- 26: the `isInsideRepo` tables and the `guardedClone`/`git` control. Counterparts: a build whose stack names `git:host.invalid:client.git@v1` while a folder of that name exists in the repo is rejected as inside the repository; a build with `GIT_ALLOW_PROTOCOL=file` in the environment still refuses the local clone the guard refuses. The remote side of the table makes no network call and stays reference-only.
- 28: the `verifyCachedLayer` and `lockedEntryFor` units, and the `gitCacheKey`/`githubCloneUrl` unit. Counterpart: after a build from a git source, the cache holds the entry named by the R1.2 formula.
- 29: the `parseAccount`, `isAccountName`, `codexPatternMatches` and `evaluateDeclarations` tables, and every `config()` (computeBuild) use. Where a `config()` case asserts what reaches generated files, it is rewritten as a CLI case that reads those files.
- Out of scope: 30, 31, 32, and the harness.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 21, 24, 26, 28 and 29 under test/acceptance import nothing from src/ directly; the R1.1 guard list drops their direct imports (24 keeps only the fake-terminal cases until R1.6)
- [x] #2 Every moved case runs under test/reference with its original name. The PR description lists each moved case with its CLI counterpart or the reason it has none
- [x] #3 `mise run test` passes; no case name is lost and the combined acceptance plus reference count is at least the baseline, with per-project counts recorded in the notes
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Implementation notes (implementer). Test-only; src/ untouched.

Moved cases (run under test/reference with their original name and body), with the CLI counterpart in test/acceptance:
- 21 `reads --stack values as path, git or github sources` (parseStackOption) -> reference/21-init-stack. Counterpart (new): `writes --stack values as path, pinned path and github sources into layer.yaml` (alz init with a bare path, path:...@ref, github:...@v; an unresolvable first entry stops resolution, so no clone is tried).
- 24 `ignores a repo-layer declaration added after the build, with one warning for both targets` and `ignores a group layer's declaration read from a cache entry and a lock, with one warning` (resolveAccounts) -> reference/24-untrusted-repo. Counterpart: the same two cases stay in acceptance with the resolveAccounts lines removed; their `alz account which` assertions (winner lines, one account-declaration-ignored warning, reason text) are the CLI side. The two fake-terminal cases stay in acceptance/24 for R1.6.
- 26 `rejects a file:/// URL whose relative spelling is a folder in the repo` and `rejects a local git: source whose text and disk disagree about '..'` (isInsideRepo line inside a CLI case) -> reference/26-commit-identity with the isInsideRepo assert; the acceptance cases stay without that line (same names).
- 26 `keeps a guarded clone from falling back to a local path, even with GIT_ALLOW_PROTOCOL=file` (guardedClone, git) -> reference. Counterpart: same name in acceptance, now a build with GIT_ALLOW_PROTOCOL=file whose stack is git:x:y@v1 with a folder x:y in the repo; the build is refused as a client layer inside the repository and nothing is written. The CLI cannot reach the guard's own refusal of a local path (an in-repo path is rejected earlier), so the guard unit itself is reference-only until the Rust clone code (R3, TASK-55.4).
- 26 `treats a git: source as remote only when no such local path exists` (isInsideRepo table) -> reference. Counterpart: same name in acceptance: git:host.invalid:client.git@v1 is fetched (not "inside the repository") with no folder, and rejected as inside the repository once a folder of that name holds a bare client layer. The remote side of the table (no network call) is reference-only: not reachable by the CLI; Rust test in R3 (TASK-55.4).
- 28 `is ok for equal content hashes and a miss for different ones` and `selects a lock entry only for the same source and the same commit` (verifyCachedLayer, lockedEntryFor) -> reference/28-cache-integrity. Counterparts: already in acceptance (edited/added-to/untouched cached layer, lock for another URL, no lock entry). The gitCacheKey/githubCloneUrl unit uses only test/spec (R1.2) and stays in acceptance; `keep two repositories whose flattened paths are equal in separate entries` asserts the cache entry names by that formula after a build from a git source.
- 29 `parses a name to the same name for every target and a map as written` (parseAccount, isAccountName) -> reference/29-account-layer. Counterpart: the existing lint accept/refuse tables.
- 29 the five `who wins` cases (config()) -> reference. Counterparts (new, same names in acceptance): build, write allowed.yaml for the winning layer, read `alz account which` (claude/codex lines and layer step). None asserted generated files; the winner is visible only through account resolution, so the CLI cases use that.
- 29 all `pattern %j against %s is %s` cases (codexPatternMatches, 15) -> reference. No CLI counterpart: the match is internal; policy patterns are covered through the CLI in 24 (`more refusals and controls for policy patterns`). Rust unit test in the account model (R2, TASK-55.3).
- 29 the five evaluateDeclarations cases -> reference. Counterparts: 24 and 31 CLI cases for refused and conflicting declarations; the pure rule gets a Rust unit test in R2/R6.

Guard list (test/guards/acceptance-imports.test.ts): 21, 26, 28, 29 moved from DIRECT to THROUGH_HELPERS (they still reach src/ through test/helpers.ts until R1.7); 24 stays in DIRECT for the fake-terminal cases. The written set is unchanged because helpers still reach src/; the direct imports are gone (grep of test/acceptance shows only 24, 31, 32 import src/).

Checks: `mise run lint` clean, `mise run test` 40 files, 958 passed, 1 expected fail, 1 skipped (960), run with TASK-55.2.4's work in the tree. Per project at the last run: acceptance 807 (806 passed, 1 skipped), reference 153 (152 passed, 1 expected fail). Baseline after R1.2 was 841 and 41 (882 total); acceptance falls because the moved tables (parametrised cases) now run in reference, total is 960. 55.2.3 alone: -23 acceptance (1 + 15 + 5 + 2), +35 reference, +5 new acceptance counterparts (21, 28 none, 26 none new names; 29 five replace the five moved, 21 one new).

Count correction: net for this task is acceptance -23 (29 cases moved out, 6 CLI counterparts added in 21 and 29) and reference +35; the 'x new' tail of the previous note is imprecise. The 26 counterparts keep their original names and replace the moved cases one for one.

Coverage check (orchestrator, 2026-11-23): vitest list at 3a8581e vs the working tree, keyed by file and case name with the acceptance/reference folder stripped: 418 names before, 468 after, none missing (the five guard names differ only by path prefix). Run after clearing leftover temp folders: 40 files, 958 passed, 1 expected fail, 1 skipped, 58 s; acceptance 807 and reference 153 cases (882 before, 960 after). Acceptance falls by design because internal cases moved to reference.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Moved the TypeScript-internal cases of 21, 24, 26, 28 and 29 to test/reference with their original names (parseStackOption, resolveAccounts, isInsideRepo, guardedClone, verifyCachedLayer, lockedEntryFor, parseAccount, the account-winner config cases, codexPatternMatches, evaluateDeclarations), adding CLI counterparts where the behaviour is visible through alz (init --stack, build with GIT_ALLOW_PROTOCOL=file, account which for the five winner cases). The moved-case list with counterparts or reasons is in the notes. Only 24 still imports src/ directly (fake terminal, R1.6); 21, 26, 28 and 29 now reach src/ only through helpers.ts (R1.7).
<!-- SECTION:FINAL_SUMMARY:END -->
