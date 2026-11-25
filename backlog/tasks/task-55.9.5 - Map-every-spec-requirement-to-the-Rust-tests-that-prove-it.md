---
id: TASK-55.9.5
title: Map every spec requirement to the Rust tests that prove it
status: Done
assignee:
  - '@implementer'
created_date: '2026-11-24T17:44:00Z'
updated_date: '2026-11-25T14:15:00Z'
labels:
  - area:spec
  - area:test
  - area:rust
milestone: m-7
dependencies:
  - TASK-55.9.3
  - TASK-55.9.4
parent_task_id: TASK-55.9
priority: medium
ordinal: 187390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
decision-39: the spec's requirements are proven by Rust tests alone, and test/ may go only when no requirement lacks one. This builds docs/spec-coverage.md from docs/LAYERING-SPEC.md and the Rust tests, never from the TypeScript tests: one row per anchor of the spec (every ## and ### heading, every paragraph that opens with a bold lead, and every numbered §13 item), naming the Rust tests (file and function) that prove it, or n/a with a reason for text that states no behaviour, planned with a task ID for behaviour the spec marks as planned, or gap with the behaviour that lacks a test. A Rust guard test keeps the matrix true as the spec and the tests change. The architect reviews the rows. The four gap subtasks then fill the gaps by spec area. Out of scope: writing gap tests.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Every anchor of the spec has exactly one row, and every row is covered, n/a, planned or gap (guard test)
- [x] #2 The guard test, run by mise run rust:check, fails when an anchor has no row, a row names an anchor the spec no longer has, or a row names a test function that does not exist (each shown failing once, in notes)
- [x] #3 The matrix states the gap count per spec section, and every gap row names the behaviour that lacks a test (file review)
- [x] #4 mise run rust:check passes
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
§13 item 19: folder ownership has no test yet; record it in the matrix (from 55.9.4).

Implementation (not checked off; the architect reviews the rows). docs/spec-coverage.md has 154 rows, one per anchor. Anchor = a ## or ### heading (§N.M), a line outside code fences that starts with a bold lead (§<enclosing section> <lead text>, §0 before the first heading), or a numbered item of §13 (§13.n). Guard: crates/alz-test-support/tests/spec_coverage.rs::the_matrix_has_a_true_row_for_every_anchor_of_the_spec (parses the spec and the matrix from the workspace root, finds test functions in crates/ by reading source for #[test] or proptest!, no test is run); its pure parts have unit tests that show each failure. AC1: guard passes, 154 anchors = 154 rows, statuses covered 114, n/a 21, planned 3, gap 16. AC2: shown failing on the real matrix by mutating a copy: a removed row gives 'missing row: §4.5'; a row renamed to §4.6 gives 'stale row: the spec has no anchor §4.6' plus the missing row; a wrong test name gives '§4.5 names crates/alz-core/src/render/protected/tests.rs::the_defaults_are_protected, which is not a test function'; also unit tests the_guard_fails_on_an_anchor_without_a_row, ..._a_row_for_an_anchor_the_spec_no_longer_has, ..._a_test_function_that_does_not_exist, ..._a_second_row_a_bad_status_and_an_empty_reason, ..._when_the_summary_is_out_of_date. AC3: summary per section and per gap task in the matrix (guard checks it); every gap row says what lacks a test. AC4: mise run rust:check passes (exit 0). Gap rows per section: §6 1, §7 1 (§7.13), §8 2, §10 4, §13 8; per task: 55.9.6 1, 55.9.7 2, 55.9.8 1, 55.9.9 12. Planned: §13.21 to 23 (TASK-50.3, 50.6, 50.5). §13 item 19 (folder ownership) was not found as a separate test: it is part of the §13.19 gap row. Findings: §6 and §7.13 have no bold leads, so each is one anchor of 345 and 740 lines and is marked gap (unaudited); §8.2 and §13.13: the Codex adapter does not exist in Rust, so the spec describes output alz does not make; the matrix marks them gap. The guard accepts a covered row only when every named test exists; it cannot check that a test asserts the requirement, which is the reviewer's job.

Correction after review: §8.2 and §13.13 are planned (TASK-18, the Codex adapter), not gaps. '§8 What an adapter may rely on' stays a gap because it is about every adapter's output keys against target-facts, not only Codex. Final counts: covered 114, n/a 21, planned 5, gap 14 (of 154). Gap rows per task: 55.9.6 1, 55.9.7 1, 55.9.8 1, 55.9.9 11. mise run rust:check passes (exit 0).
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
docs/spec-coverage.md maps all 154 spec anchors (114 covered, 21 n/a, 5 planned, 14 gap); the guard spec_coverage.rs keeps rows, tests and summaries true. Owner approved; merged in #27 (e74274c).
<!-- SECTION:FINAL_SUMMARY:END -->
