---
id: TASK-55.9.1
title: Move the test fixtures to a workspace-level fixtures/ folder
status: Done
assignee:
  - '@implementer'
created_date: '2026-11-24T17:43:00Z'
updated_date: '2026-11-24T19:01:00Z'
labels:
  - area:rust
  - area:test
milestone: m-7
dependencies: []
parent_task_id: TASK-55.9
priority: medium
ordinal: 183390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The Rust tests read test/fixtures in about 116 places (include_str! paths and fixture roots in alz-core, alz-io and alz), and R8 deletes test/ (decision-39). Moving every fixture with git mv to fixtures/ now lets later subtasks write new fixtures in their final place. The vitest harness finds fixtures through one root in test/helpers.ts; repointing that line is the only edit to test/, so the acceptance suite keeps running as the safety net until the deletion subtask. Out of scope: fixture contents, deleting fixtures (the deletion subtask removes those no Rust test reads), and the spec text (the spec subtask).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 git grep -n 'test/fixtures' -- crates mise.toml .github .layer scripts prints nothing (grep)
- [x] #2 Every fixture moved unchanged: git diff main -M --name-status -- test/fixtures fixtures lists only R100 lines (git diff)
- [x] #3 mise run rust:check passes
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Implementation (commit f49deca on chore/fixtures-task-55.9.1). All 32 folders under test/fixtures moved with git mv to fixtures/ (294 files). AC1: git grep -n 'test/fixtures' -- crates mise.toml .github .layer scripts prints nothing. AC2: git diff main -M --name-status -- test/fixtures fixtures lists 294 R lines, 0 not R100. AC3: mise run rust:check exit 0 (fmt, clippy -D warnings, nextest, doctests) after the move. Paths changed: 14 Rust test files in alz-core/alz-io/alz (include_str! and CARGO_MANIFEST_DIR joins, same depth so only the segment changed), crates/alz/tools/golden-trees.mjs, crates/alz/tests/fixtures/golden/README.md, .layer/{instructions/50-repo.md,rules/tests.md,skills/add-acceptance-test/SKILL.md}, biome.json and tsconfig.json excludes. AGENTS.md, .claude/ copies and layer.lock regenerated with the Rust alz build (run from a scratch copy because the personal layer path is relative to the checkout). Not done on purpose: test/helpers.ts root (owner said no TypeScript test changes; the vitest suite will not find fixtures until deleted in 55.9.11) and docs/LAYERING-SPEC.md line 3940 (spec subtask 55.9.4).
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
All 32 fixture folders (294 files) moved from test/fixtures to fixtures/ as pure renames; the Rust tests and .layer/ read them there. Merged in #18 (cb116c2).
<!-- SECTION:FINAL_SUMMARY:END -->
