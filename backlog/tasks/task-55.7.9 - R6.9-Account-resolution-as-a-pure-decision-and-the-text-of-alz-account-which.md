---
id: TASK-55.7.9
title: 'R6.9: Account resolution as a pure decision, and the text of alz account which'
status: Done
assignee:
  - '@implementer'
created_date: '2026-11-24T08:16:00Z'
updated_date: '2026-11-24T12:48:00Z'
labels:
  - area:rust
  - area:accounts
milestone: m-7
dependencies:
  - TASK-55.7.8
parent_task_id: TASK-55.7
priority: high
type: task
ordinal: 168390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Plan for this step (§7.13 "Resolution", "A named account that cannot apply fails closed", "Values set outside alz"; doc-6 §3.2).
- alz-core account/resolution.rs: per target, first match wins: the use choice, the user config's longest folders entry, the layer when allowed and readable, the default account. A value without its marker, or with another, is foreign and replaces steps 3 to 5, never 1 and 2; a set but empty value is foreign; a marker without its variable is stale.
- Applied { Folder, Default, Kept(Foreign) } and FailsClosed (missing folder, cache miss with the recorded account, changed, source gone, conflict, unreadable manifest, configured folder, invalid name, relative home), matched exhaustively; what the hook applies and the fix for each.
- account/which_report.rs: the lines of R6.2; the exit code (0 when every target resolves, 1 when any fails closed).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Property: a named account that cannot apply never resolves to the default account or to a newly declared account (proptest over generated facts)
- [x] #2 Property: the first step that names an account wins, a foreign value replaces steps 3 to 5 but never 1 and 2, and a target the allowed entry does not name is skipped without failing (proptest)
- [x] #3 Property: no report line holds a control or format character or a name that breaks the name rule, whatever the paths and sources hold (proptest)
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
From R6.7 (2026-11-24), lead decisions: (1) a working directory inside a configured folder is placed by that folder's real path, nothing below it is looked at, so an allowed.yaml root strictly below the configured folder is not matched (stricter); R6.16's spec pass or R6.9 adds that sentence to §7.13; use RootSearch::known_directory() for the folders rule match. (2) The account warnings must print exactly the spec's §10.3 lines (the reference's): no file:line:column prefix, keys quoted as the spec writes them — align crates/alz-io/src/account/{files,resolved}.rs and the alz-core account readers with §10.3, with tests against the spec's strings. (3) account-default-name-folder is produced here.

R6.9 (implementer, 2026-11-24), uncommitted on feat/rust-r6-task-55.7. Code: alz-core account/resolution/{facts,outcome,steps,tests}.rs (resolve(&Facts, &impl Stack) -> Resolution; Applied{Folder,Default,Kept(Foreign)}, FailsClosed{InvalidName,HomeNotAbsolute,MissingFolder,CacheMiss,Changed,SourceGone,Conflict,Manifest,ConfiguredFolder} with hook() and fix(); the layers are read lazily once through the Stack trait and never for a root that is not allowed; folders rule matches known_directory by whole segments, longest entry, a shorter one never tried) and account/which_report.rs (+tests): the §10.3 lines, exit 0/1, warnings = earlier + resolution's (account-foreign, account-declaration-ignored, account-changed, account-default-name-folder). New Code variants AccountForeign, AccountDeclarationIgnored, AccountChanged; Message::and; Quoted (class 1 in double quotes); Ignored gained source. Alignment to §10.3 (lead decision 2): account warnings carry no file:line:column and no path (Where is the bare file name), keys in double quotes, top-level shape texts are the spec's, account-home text uses ', so'; alz-io files.rs parses once and reports invalid YAML as FileStatus::Ignored(IgnoreReason::NotValidYaml) with 'account-file-ignored <file> (<path>) is ignored: it is not valid YAML' (spec 'never (read)'). StateDir: a file that is invalid YAML is now refused as Place(NotValidYaml) (was NotAList); valid YAML that is not a list stays NotAList (tests: a_file_that_is_not_a_list_or_holds_an_ignored_entry_is_refused_by_position and a_file_that_is_not_valid_yaml_is_an_ignored_file_and_refused_with_that_reason). Evidence: AC1 proptest a_named_account_that_cannot_apply_never_resolves_to_the_default_or_a_new_account (+ unit tests per failure reason); AC2 proptest the_first_step_that_names_an_account_wins plus unit tests for foreign vs steps 1-2 and for a target the entry does not name; AC3 proptest no_line_holds_a_control_or_format_character (lines and warnings; names are AccountName only, an invalid explicit name is never printed). Checks: mise run rust:ci exit 0 (fmt, clippy -D warnings, tests, doc, MSRV, deny, audit); cargo test --workspace 1964 passed 0 failed; mise run lint exit 0. NOT done: parity run of src/ for fixtures of tests 30 and 31 (expected lines were taken from test/acceptance/31-account-which.test.ts instead; the real parity comes when R6.10 wires the command and the acceptance suite runs against ALZ_BIN); the adapter from alz-io types (Loaded, ResolvedUserConfig, AccountFolderNames, CachedDeclarationReader, find_root) to Facts and Stack is R6.10. Spec: add to §7.13 the sentence that a working directory inside a configured folder is placed by that folder's real path, so an allowed.yaml root strictly below it is not matched.

Follow-up (2026-11-24): Quoted keeps the R2 key rule. Quoted::key (used for accounts.yaml keys) strips, cuts after 64 code points counted after stripping, puts the ellipsis inside the double quotes, escapes " and \ with a backslash and never counts the added backslashes; Quoted::from_untrusted (paths, sources, layer names) strips and escapes but cuts nothing, as the reference's quote() does. Tests: a_quoted_key_of_65_code_points_is_cut_at_64_with_an_ellipsis_inside_the_quotes and a_double_quote_and_a_backslash_inside_a_quoted_text_get_a_backslash_and_the_cut_ignores_them. rust:ci exit 0, lint exit 0 after the change.

Verified by the lead (2026-11-24): mise run rust:ci and lint exit 0, src/ unchanged. Accepted: invalid YAML in an account file is the ignored reason 'it is not valid YAML' (spec), and the StateDir test change; account keys print with the spec's double quotes and the owner's 64 code point cut (Quoted::key); paths, sources and layer names are stripped and not cut, as the reference does. The parity run against tests 30 and 31 comes with R6.10.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
resolve(&Facts, &impl Stack) decides each target's account purely: the steps in order, a named account that cannot apply fails closed with its reason and fix and never falls back, layers read once and never for a root that is not allowed; which_report renders the §10.3 lines; account warnings print as the spec writes them.
<!-- SECTION:FINAL_SUMMARY:END -->
