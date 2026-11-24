---
id: TASK-55.9
title: 'R8: Cut over to the Rust binary'
status: To Do
assignee: []
created_date: '2026-11-22T12:54:00Z'
updated_date: '2026-11-24T17:49:00Z'
labels:
  - area:rust
milestone: m-7
dependencies:
  - TASK-55.7
parent_task_id: TASK-55
priority: medium
ordinal: 41390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
doc-5 phase R8, re-scoped by decision-39: alz becomes Rust only. Rust tests written from docs/LAYERING-SPEC.md prove every requirement; the TypeScript implementation, its tests and the Node toolchain are deleted; the spec, .layer/ and the docs state only what alz does. The vitest acceptance suite keeps running in CI as a safety net until the coverage matrix shows no gap, but its code and tests are never read as a reference (only listed for deletion). The subtasks are ordered by dependency. Out of scope: distribution and the first release (R7, TASK-55.8, which now waits for R8), and npm (no wrapper; the owner removes the package).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Every subtask is Done
- [ ] #2 git ls-files lists no .ts, .mts, .cts, .js, .mjs or .cjs file, no package.json, package-lock.json, tsconfig.json, biome.json or vitest.config.ts, and nothing under src/ or test/ (grep)
- [ ] #3 docs/spec-coverage.md maps every heading, rule paragraph and §13 item of the spec to Rust tests with no gap row, and its guard test passes in mise run rust:check
- [ ] #4 mise run alz:check passes with the Rust binary, and no mise task, hook or CI job runs npm, npx or node (grep)
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
2026-11-24, replanned under decision-39 (architect). Former criterion #1 is split into #2 and #4; former #2 (Rust tests cover the spec sections) is #3, now measured by a coverage matrix built from the spec, not from the TypeScript suite; former #3 (the release describes every security fix it ships) moves to R7's first release, TASK-55.8.4, because without an npm wrapper the first release, not a pull request, replaces 0.1.0 for users. R8 no longer depends on R7: nothing in it needs distribution, and the first release waits for R8 instead.

Lead (2026-11-24), on the owner's instruction (no TypeScript tests are run, everything in TypeScript is deleted, Rust tests are written from the spec): no vitest safety net. Order: 55.9.1 fixtures and 55.9.2 alz:build through the Rust binary, then 55.9.10 (.layer, mise and CI Rust only) and 55.9.11 (delete all TypeScript, test/ included), then 55.9.3 and 55.9.4, then the matrix 55.9.5 and the gap tests 55.9.6 to 55.9.9. 55.9.10 no longer waits for the gap tests; no criterion asks for mise run test or the acceptance suite to pass.
<!-- SECTION:NOTES:END -->
