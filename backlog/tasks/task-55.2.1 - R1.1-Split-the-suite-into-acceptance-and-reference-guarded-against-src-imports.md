---
id: TASK-55.2.1
title: >-
  R1.1: Split the suite into acceptance and reference, guarded against src
  imports
status: To Do
assignee: []
created_date: '2026-11-22T17:54:00Z'
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
- [ ] #1 `mise run test` runs the acceptance and reference projects and passes. The notes record the baseline per project (files, cases, duration), and later subtasks compare against it
- [ ] #2 The guard test passes with a written list of violating acceptance files. The list has the eight direct importers (21, 24, 26, 28, 29, 30, 31, 32) and every file that reaches src/ through test/helpers.ts or test/account-workspace.ts
- [ ] #3 Positive controls in the guard test find a type-only import, an `await import("../../src/x")` inside a `vi.mock` factory, an import through a test helper, and a `join(SRC, ...)` entry point; each one makes the check fail
<!-- AC:END -->
