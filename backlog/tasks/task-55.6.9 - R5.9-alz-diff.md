---
id: TASK-55.6.9
title: 'R5.9: alz diff'
status: Done
assignee:
  - '@implementer'
created_date: '2026-11-23T18:09:00Z'
updated_date: '2026-11-24T04:19:00Z'
labels:
  - area:rust
  - area:cli
milestone: m-7
dependencies:
  - TASK-55.6.8
parent_task_id: TASK-55.6
priority: high
type: task
ordinal: 141390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Plan for this step (§10).
- alz-core diff.rs: a unified diff of two byte strings as owner question 3 decides, with three lines of context and `a/<path>` and `b/<path>` headers; content that is not valid UTF-8 or holds NUL is `Binary file <path> changes`.
- command/diff.rs: the pipeline, then one diff per change and for the lock, a note per collision on standard error, `no changes` when there is none; exit 1 only on diagnostics errors.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Applying the diff to the old text gives the new text, for generated text pairs (proptest)
- [x] #2 alz diff after editing a generated file shows a diff of that file, and on a built repository prints no changes (assert_cmd tests)
- [x] #3 The diff cases of test 28 pass against ALZ_BIN (vitest run)
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Implementation (not committed, ACs not checked).
Files: crates/alz-core/src/render/diff.rs (+ diff/myers.rs, diff/tests.rs), render.rs (one module line); crates/alz/src/command/diff.rs, command.rs, cli.rs (Diff variant), main.rs (dispatch), command/pipeline.rs (plan_with_disk: plan plus the Disk, plan delegates to it); crates/alz/tests/diff.rs.
Design: alz-core compare(old,new) -> Comparison {Unchanged, Binary, Text(hunks)}; Myers linear-space middle snake with prefix/suffix trimming, removals before additions per run; render(path) strips every line and the path with class 1 (the diff itself works on raw lines). The command takes old bytes from the writer's Disk (Observed::Regular), new bytes from the plan; no re-read, no write. Notes go to stderr as raw 'note [collision] ...' lines (no Note level in Diagnostics).
AC1: proptests in render/diff/tests.rs: applying_the_hunks_to_the_old_text_gives_the_new_text, equal_texts_give_no_diff_and_the_same_texts_the_same_diff (determinism), the_edit_script_is_a_shortest_one_with_removals_first (kept count == LCS by DP), the_hunk_counts_match_their_rows_and_context_is_at_most_three_lines; plus unit tests for the spec format (context, created/deleted, no-newline marker, join at 6 vs 7 unchanged lines, binary by NUL/UTF-8/8192 probe, class 1 stripping). 15 tests pass.
AC2: crates/alz/tests/diff.rs (assert_cmd, 6 tests pass): first build shows created files and layer.lock and writes nothing; built repo prints 'no changes'; edit of AGENTS.md shows a diff that undoes it; control chars never reach stdout; collision note on stderr with exit 0; no manifest exits 1 with empty stdout.
AC3: release build, ALZ_BIN=target/release/alz vitest run --project acceptance test/acceptance/28-cache-integrity.test.ts: only file with a diff case ('"diff"' grep). Both diff cases pass (rebuilds from an untouched cache entry; fails the build when a cached git layer was edited). File total 65 passed, 12 failed; the 12 do not run diff (build/lint/init wording and cache-link cases owned by other tasks, e.g. 'is not a plain directory', 'not a valid lock file', 'unsafe entry'). No case marked reference drift.
Checks: mise run rust:check green (alz-core lib 1184 tests); rust:ci fmt, clippy, tests, doc, msrv, deny pass; rust:audit fails only on 'couldn't check if the package is yanked' (network timeout). mise run lint green.
Open: spec silent on a mode-only Update (bytes equal) and on .gitignore changes: both print no patch, and 'no changes' is printed only when the plan has no changes and the lock is unchanged (as the reference). Worst-case Myers time is O(ND) on 1 MiB inputs with very many differing lines.

Revision after lead decisions (not committed).
1. Notes: Level::Note and Diagnostic::note added to alz-core diagnostic.rs (prints 'note [code] ...', counted neither as error nor warning; unit test). alz diff builds one Diagnostics of collision notes and prints it through the normal Display, so the path is stripped like any diagnostic; no raw lines.
2. Budget: myers::script takes a cost cap (render::diff::COST_CAP = 2^24); after setting aside the shared first/last lines, the search depth is limited to cap/(old+new) edits; past it the lines between the shared ends are removed whole and the new ones added whole (one change run in one hunk, still a valid patch). Tests: a_diff_that_gave_up_the_search_still_applies_cleanly (proptest, tiny budget), a_budget_script_has_every_element_once_and_no_addition_before_a_removal (proptest), past_the_budget_the_middle_is_replaced_whole_between_the_shared_ends, crafted_texts_of_about_one_mebibyte_are_compared_in_bounded_time (all-lines-differ pair and a pair near the cap, 30 s timeout, runs in under 1 s). Spec sentence added to 10.3 diff.
3. Reference behaviour (ran dist/cli.js in a scratch copy of 02-instructions): mode-only update prints the two header lines and no hunk (jsdiff also prints a '====' line, dropped as the spec's format has none); an empty file / equal bytes prints headers only (checked with jsdiff createTwoFilesPatch); a .gitignore change prints nothing and 'no changes' still prints. Rust now matches: Comparison::render returns the headers for Unchanged; command prints it for every change. Tests: crates/alz/tests/diff.rs a_change_of_the_executable_bit_alone_prints_its_headers_and_no_hunk and a_missing_gitignore_block_is_not_shown_and_leaves_no_changes; core equal_texts_have_no_diff (headers for equal and empty texts). Spec 10.3 gets two sentences (equal bytes, .gitignore) and 'no changes' is now worded as no generated file changes and layer.lock current.
Checks: mise run rust:ci all green including rust:audit on retry; mise run lint green; acceptance 28-cache-integrity diff cases still pass (2 of 2).

Verified by the lead (2026-11-24): mise run rust:ci and mise run lint exit 0, src/ unchanged. Lead decisions applied: collision notes go through Level::Note (stripped like every diagnostic); the Myers search has a cost budget (2^24 / lines; past it one replacing hunk, two crafted 1 MiB texts in under a second); mode-only and empty-file changes print headers only and .gitignore changes are not shown, as the reference does.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
alz diff prints alz's own unified diff of what alz build would change, from the bytes the writer observed and the plan, without writing: Myers with a cost budget, three lines of context, binary detection, stripped output, collision notes at level note.
<!-- SECTION:FINAL_SUMMARY:END -->
