---
id: TASK-55.2.1
title: >-
  R1.1: Split the suite into acceptance and reference, guarded against src
  imports
status: Done
assignee:
  - '@implementer'
created_date: '2026-11-22T17:54:00Z'
updated_date: '2026-11-22T18:13:00Z'
labels:
  - area:test
  - area:rust
milestone: m-7
dependencies: []
references:
  - backlog/docs/doc-5 - Rust-port-plan.md
  - test/import-graph.ts
  - vitest.config.ts
parent_task_id: TASK-55.2
priority: high
type: task
ordinal: 71390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
First step of R1 (TASK-55.2). Later subtasks move cases out of the acceptance path, so the boundary and a check on it come first.

- vitest.config.ts gets two projects. `acceptance` runs test/acceptance/** and is the language-neutral oracle. `reference` runs test/reference/** and test/guards/**: TypeScript-only tests that may import src/ and are deleted with src/ at cutover (R8). `mise run test` keeps running both, with the same 20 s test timeout.
- test/reference/in-process.ts: today's in-process driver (the body of `layers()` with `run` and the optional fake `Terminal`), for cases that later move there. In this task it is a copy; helpers.ts does not change.
- test/import-graph.ts gains a walk that starts at any test file. It follows value and type-only imports, `import()` literals (including those inside `vi.mock` factories) and `require`, and reports repository-relative paths. The current `valueImports(entry)` and `SRC` keep their behaviour, because 31 and 32 use them.
- test/guards/acceptance-imports.test.ts: for every file under test/acceptance, reaching src/ in any way is a violation: an import of any kind, transitively through test helpers, or a use of `SRC` (an esbuild entry point under src/). The test asserts that the set of violating files equals a written list. While other subtasks convert files, the list must shrink, and a file that is fixed but still listed also fails the test. R1.7 empties the list.
- Out of scope: moving or changing any acceptance case; the harness (R1.7).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 `mise run test` runs the acceptance and reference projects and passes. The notes record the baseline per project (files, cases, duration), and later subtasks compare against it
- [x] #2 The guard test passes with a written list of violating acceptance files. The list has the eight direct importers (21, 24, 26, 28, 29, 30, 31, 32) and every file that reaches src/ through test/helpers.ts or test/account-workspace.ts
- [x] #3 Positive controls in the guard test find a type-only import, an `await import("../../src/x")` inside a `vi.mock` factory, an import through a test helper, and a `join(SRC, ...)` entry point; each one makes the check fail
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Implementation (2026-11-22):
- vitest.config.ts: two inline projects with extends: true (the 20 s testTimeout is inherited; checked with a 6 s wait in each project). acceptance = test/acceptance/**; reference = test/reference/** and test/guards/**.
- test/reference/in-process.ts: layersInProcess(), a copy of the body of layers() with the optional fake Terminal. helpers.ts is unchanged. Nothing uses it yet.
- test/import-graph.ts: added REPO, referencedSpecifiers(text) and reachableModules(entry, root = REPO). The walk follows value and type-only imports, export ... from, export * from, side-effect imports, import() literals (including inside vi.mock factories), require, and the vi.mock/doMock/importActual/importMock literals (an addition beyond the task text). It reports root-relative paths, reports packages as written and unfollowable specifiers as <dynamic import>/<dynamic require>. importedSpecifiers, valueImports and SRC are unchanged.
- test/guards/acceptance-imports.test.ts: a file is a violator when its reach contains src/**, test/import-graph.ts (the door for SRC and valueImports) or an unfollowable specifier. The written list is 30 files: the 8 direct importers (21, 24, 26, 28, 29, 30, 31, 32) plus 22 that reach src/ only through test/helpers.ts (01-12, 14-20, 22, 23, 25). Only 27 is clean. Controls run against throwaway repos in the temp folder: type-only import, await import inside a vi.mock factory, import through a helper, join(SRC, ...) entry point, plus a clean negative control, a fixed-but-still-listed control and a path-reporting check. The controls were written first against a stub walk that returned an empty set: 6 of 8 failed with 'expected [] to deeply equal [...]', the clean and stale controls passed as they should.
- Test-time finding: test/account-workspace.ts and test/allow-driver.ts are importers of src/ but are not acceptance files; they are reached by 24, 30, 31, 32 which are direct importers anyway, so no extra list entries.

Baseline (before R1.2; later subtasks compare against it), mise run test on macOS, Node 22:
- acceptance: 31 files, 841 cases (840 passed, 1 skipped), 56.3 s alone.
- reference: 1 file (the guard), 8 cases, 0.37 s alone.
- both together (mise run test): 32 files, 849 cases (848 passed, 1 skipped), 55.2 s.
- Before the split the suite was 31 files, 841 cases.
- No git stall (TASK-54) seen in 3 full runs.

Correction: the git stall was not seen in 2 full runs (the acceptance project alone, and mise run test), not 3. The pre-split count of 31 files and 841 cases is the acceptance project's count, not a separate run.

Orchestrator rerun: mise run test 32 files, 848 passed, 1 skipped (57.8 s); mise run lint clean.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
vitest now has two projects: acceptance (test/acceptance/**, the language-neutral oracle) and reference (test/reference/**, test/guards/**, TypeScript-only, deleted at cutover). test/reference/in-process.ts copies today's in-process driver for later moves. test/import-graph.ts gains reachableModules, a transitive walk over every import form including vi.mock/import() literals and require. test/guards/acceptance-imports.test.ts asserts the exact set of acceptance files that reach src/ (30 files: 8 direct, 22 through test/helpers.ts; only 27 is clean), failing on both unlisted and stale entries, with positive controls run in throwaway repos. Baseline: acceptance 31 files / 841 cases / ~56 s; reference 1 file / 8 cases.
<!-- SECTION:FINAL_SUMMARY:END -->
