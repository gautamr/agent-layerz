---
id: TASK-55.5.12
title: 'R4.12: Passthrough files and their protected-variable checks'
status: Done
assignee:
  - '@implementer'
created_date: '2026-11-23T12:28:00Z'
updated_date: '2026-11-23T15:04:00Z'
labels:
  - area:rust
  - area:merge
  - area:security
milestone: m-7
dependencies:
  - TASK-55.5.11
parent_task_id: TASK-55.5
priority: high
type: task
ordinal: 125390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Plan for this step (§7.11 and the passthrough part of §7.9).
- item/passthrough.rs: targets/<target>/<path> per target; the most specific layer wins per path; a passthrough warning per use. A path with a non-ASCII character is refused for every target.
- Checks for every target, paths compared with ASCII case folding: paths ending in .claude/settings.json or .claude/settings.local.json, every .mcp.json and every .claude-plugin/*.json are parsed with alz's JSON tree, and the root env, the env of each mcpServers entry and any nested env mapping are checked; .claude/agents/**/*.md frontmatter mcpServers is checked; a *.toml file is refused by the text rule of §7.9 (shell_environment_policy, a whole-name mention, \u, \U or \x escapes, a trailing backslash). A covered file that does not parse is refused.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Each passthrough case of §7.9 is refused with protected-variable and no value, for every target, with case-folded path matching (table tests)
- [x] #2 A non-ASCII passthrough path is refused, and a covered JSON file or agent frontmatter that does not parse is refused (unit tests)
- [x] #3 The most specific layer wins per path and each use warns (unit test), and the per-path merge is associative (proptest)
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Architect (2026-11-23, R4.1): spec §4.1 and §7.11 decide the case of a folder in targets/ whose name is not a known target. It is an error, not ignored: each file below such a folder is a passthrough error that names the file. The reference merges files under any folder name, warns and writes none of them, so the Rust behaviour is reference drift (TASK-55.5). A file directly in targets/ is no item. R4.2's ItemPath gives Other for both; R4.12 turns the first into the error. Add a unit test: targets/claud/x.json gives a passthrough error naming the file, and targets/README.md gives no diagnostic.

Implementer (2026-11-23, R4.12). Files: alz-core item/passthrough.rs (+ passthrough/{coverage,check,toml_text,tests}.rs), merge/passthrough.rs (+ passthrough/{laws,tests}.rs); edited item.rs, item/protected_variable.rs (ServerField::File, FILE_* key paths, refused/unchecked visible to item), merge.rs, merge/stack.rs (Merged::passthrough(), PassthroughState in the fold), merge/provenance.rs (Traced::with_provenance), diagnostic.rs (Code::Passthrough). Checks: mise run rust:ci exit 0 (fmt, clippy -D warnings, tests, doc, MSRV, deny, audit); mise run lint exit 0.
AC1: item/passthrough/tests.rs every_passthrough_form_is_refused_for_every_protected_variable_and_never_prints_a_value: 17 forms (settings.json, settings.local.json, .mcp.json, case-variant .Claude/Settings.JSON and .Codex/Config.toml, any depth, plugin manifest and nested env, .toml under both targets and outside .codex, .claude/agents/**.md under both targets with map and list mcpServers) x 6 variables, each: protected-variable error at personal:<file> naming the variable in its fixed spelling, no value (canary) printed, no item. Also fixed_spelling test (lower/mixed-case keys), non-plain names (NAME=x, NUL) refused without printing them. coverage.rs tests: path coverage by ASCII case folding, any depth.
AC2: a_path_with_a_non_ascii_character_is_refused_for_every_target (7 paths of test 24 incl. non-ASCII target folder), a_covered_file_that_cannot_be_parsed_is_refused_because_it_cannot_be_checked (bad JSON, duplicate key, env list, \u \U \x escapes, line-ending backslash, unparsable and unclosed agent frontmatter), a_toml_file_that_is_not_utf8_cannot_be_checked.
AC3: merge/passthrough/tests.rs the_most_specific_layer_wins_per_path_and_other_paths_stay, every_use_warns_even_a_file_that_a_more_specific_layer_replaces; merge/passthrough/laws.rs proptest: merging_is_associative, the_empty_state_is_the_identity_on_both_sides, a_fold_keeps_the_most_specific_file_per_path_and_every_layer_that_had_it (content and provenance). Lead's decision: a_file_below_a_folder_that_is_not_a_target_is_an_error_naming_the_file (targets/claud/x.json: error [passthrough]), a_file_directly_in_targets_is_no_item_and_gives_no_diagnostic (targets/README.md), plus merge-level test that the error stops the merge.
Case-to-test mapping: tests 11 and 18 have no passthrough case (grep: only test 24 uses passthrough). Test 24: the 'FORMS' passthrough rows -> the 17 forms above; 'refuses ... because it cannot be checked' passthrough rows -> the cannot-be-checked test; 'still builds with' passthrough rows (FOO env, config.toml with homepage) -> files_that_name_none_of_the_variables_still_build_with_one_warning_each (warning only, no error); 'variable name that is not a plain name' passthrough rows -> the non-plain-name test; 'passthrough paths that a file system could fold' (7 rows) -> the non-ASCII test; 'config.toml whose only refusable content is the policy table' -> covered by the policy-key mention in toml_text tests and form 'config.toml, policy'. Cases drive alz build and pass in R5.
Spec gaps and choices (spec not edited): (1) The text of the passthrough warning and of the unknown-folder error is not in the spec; used 'raw passthrough to <path>; consider turning it into a first-class item' (the reference's text) and 'this file is below a folder of targets/ that is not a target: claude or codex (§7.11)'. (2) A path with a non-ASCII character below an unknown folder (test 24 'a non-ASCII target name') must give the protected-variable non-ASCII error (test expects it); the spec's wording 'passthrough path' covers it only if every folder counts: it gets BOTH the passthrough error and the non-ASCII protected-variable error. Suggest §7.9 say 'for every folder of targets/'. (3) A .toml that is not valid UTF-8 is refused as 'cannot be checked' (spec silent). (4) The TOML text rule reports one error per distinct protected variable mentioned (fixed order) plus one for the policy key and one for an escape; the reference reports the first mention only; the verdict is the same. (5) mcpServers of a JSON or agent file that is neither a mapping nor a list (a plugin manifest names a file) is not refused, as the spec's 'Not checked' says and as the reference does; the root of a JSON file that is not an object is not checked. (6) No nesting-depth refusal for nested env (the reference refuses past depth 32); the JSON reader's 128 limit bounds the recursion. (7) A refused file gives errors only, no warning (the reference also warns). (8) Passthrough items are keyed per target by the path inside the target's folder; ItemPath is unchanged (Other for unknown folders; the reader tells the two apart by the targets/<folder>/ prefix). The unreachable SafeRelPath refusal of the inner path keeps the reader total.

Verified by the lead (2026-11-23): mise run rust:ci exits 0, lint passes, src/ and test/ unchanged. Choices accepted and collected for the spec.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Passthrough files under targets/claude and targets/codex merge per path, the most specific layer winning, each use a passthrough warning; a folder of targets/ that is not a target is a passthrough error; protected variables are refused in every covered passthrough form (settings, MCP and plugin JSON, TOML text, agent frontmatter) at any depth and in case-variant paths, and files that cannot be checked are refused; test 24's passthrough rows have Rust tests with the same verdicts.
<!-- SECTION:FINAL_SUMMARY:END -->
