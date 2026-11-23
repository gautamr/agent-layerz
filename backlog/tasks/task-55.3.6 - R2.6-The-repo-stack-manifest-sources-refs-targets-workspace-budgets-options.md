---
id: TASK-55.3.6
title: >-
  R2.6: The repo stack manifest: sources, refs, targets, workspace, budgets,
  options
status: Done
assignee:
  - '@implementer'
created_date: '2026-11-23T05:18:00Z'
updated_date: '2026-11-23T06:25:00Z'
labels:
  - area:rust
  - area:resolve
milestone: m-7
dependencies:
  - TASK-55.3.5
parent_task_id: TASK-55.3
priority: high
type: task
ordinal: 95390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Plan for this step. Spec §5.2, §6 and R2.1; fixes TASK-61 in the model. Modules `crates/alz-core/src/stack.rs` with `stack/{source,git_ref,manifest}.rs`, and `path.rs` with `SafeRelPath` (shared with R2.7 and alz-io).

Types:
- `GitRef`: no leading `-`, no `..`, only `[A-Za-z0-9._/+-]`; YAML text only, so `ref: [x]` and `ref: 1.10` are errors naming `stack[i].ref` (TASK-61), never printing the value.
- `Source = Path { dir } | Git { url: GitUrl, git_ref } | GitHub { owner, repo, git_ref }`, parsed from `path:`, `git:` and `github:`. `GitUrl` refuses credentials as §5.2 states (TASK-71's rule; its CLI cases need `build` and close in R3).
- `StackEntry { source, pin: Option<GitRef> }`, with R2.1's rule for which sources take `ref:`.
- `Manifest { stack: Stack<Declared>, targets: BTreeSet<TargetName> (default claude), workspace: Workspace { hub, add_dirs } (relative dirs, hub first, no duplicates), protected_paths, mcp_enable, budgets (whole numbers, spec defaults), options }`. `Stack<Declared>` is the first state of doc-6 §3.3; `Resolved` is R3's.
- `SafeRelPath`: `/`-separated, valid UTF-8, no empty, `.`, `..` or `.git` segment, no backslash, no NUL. Its `Ord` is `str`'s, which is spec §6 name order.

Laws (proptest): a valid source prints back to its input byte for byte and parses again to the same value (the lock and allowed.yaml store that text); `SafeRelPath` order equals the order of UTF-8 bytes and of code points (§6).

Untrusted input: the whole manifest.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 ref: [x], ref: 1.10, a ref starting with - and a ref holding .. are refused with an error that names the entry and not the value, and ref: "1.10" is accepted (unit tests, TASK-61 in the model)
- [x] #2 Every source form of spec §5.2 parses to its variant and prints back byte for byte; credentialed git URLs are refused as §5.2 states, and a user name alone in ssh:// and the scp-like form is accepted (unit tests)
- [x] #3 The name-order property test for SafeRelPath passes, and unknown fields, an empty stack and a file without stack are refused with the field path (unit tests)
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
R2.6 implementation (not committed, ACs not checked). rust:ci exit 0 (fmt, clippy -D warnings, tests, doc, MSRV, deny, audit); alz-core 178 unit tests + doc tests pass.
Files: crates/alz-core/src/path.rs (SafeRelPath), stack.rs + stack/{git_ref,source,entry,workspace,budgets,manifest}.rs, stack/source/tests.rs, stack/tests.rs. Shared edits: lib.rs (pub mod path; pub mod stack;), schema.rs (Refusal variants GitRef/Source/Target/WorkspaceDir/Budget/Stack, manifest_diagnostic(s), all/refine_node/refine_list), diagnostic.rs (Code::Source), target.rs (TryFrom<&str>, TargetError), yaml/fields.rs (boolean, items, text_of, NotBoolean), docs/LAYERING-SPEC.md 5.2 (new 'Source forms' paragraph).
AC1: stack/tests.rs a_ref_that_is_a_list_a_number_or_a_mapping_is_refused_naming_the_entry_and_not_the_value ([x], 1.10, 1, {a: b}, true -> 'stack[0].ref must be text; put the value in quotes...'); a_ref_that_breaks_the_rule_is_refused_naming_the_entry_and_not_the_value (-x, --upload-pan, a..b, empty, value never printed); a_quoted_ref_that_looks_like_a_number_is_kept_as_written (ref: "1.10" accepted); stack/git_ref.rs tests.
AC2: stack/source/tests.rs every_source_form_parses_to_its_variant, every_source_prints_back_byte_for_byte..., proptest a_valid_source_prints_back_to_its_input, credentials_in_a_git_url_are_refused (https user:token, https token, ssh user:pw, scp-like user:pw), a_user_name_alone_is_accepted_in_ssh_and_scp_like_urls; stack/tests.rs a_git_source_with_credentials_is_the_error_source_without_a_location_or_the_token (exact line 'error [source] a git: source in layer.yaml holds credentials ...', token absent).
AC3: path.rs tests name_order_is_code_point_order_and_utf8_byte_order (proptest, plus a-b < a/b); stack/tests.rs a_file_without_stack_an_empty_stack_and_a_malformed_stack_are_refused_with_the_field_path, unknown_fields_are_refused_as_quoted_keys_at_every_level.
Deviations from plan: no Stack<Declared> generic, DeclaredStack is a plain non-empty type (doc-6 3.3: separate types when states carry different data; Resolved holds dirs). StackEntry.pin is kept only for path: sources (a ref: next to github:/git: is checked then dropped, as spec 5.2). Source is the text-only enum and round-trips alone.
Choices the spec did not state (added to 5.2 'Source forms'): path: must be non-empty; github owner/repo charset [A-Za-z0-9._-], not leading -, not ./..; scp-like user part is up to the last @ before the first /, so host:path@x/y is refused as credentials (false positive accepted); http(s) scheme case-insensitive; authority ends at / \ ? #. Workspace: a leading \ is read as / and so refused as absolute (reference keeps /x). Null (empty value) for a text, list, mapping, bool or number field is an error, as R2.4/R2.5 helpers already do; the reference treats null as absent. targets: [] is allowed (empty set).
R2.11 acceptance adjustments: manifest errors print 'error [manifest] layer.yaml:<l>:<c>: <field path> ...' with line:column; refused ref/workspace/unknown-field values are not printed (reference drift TASK-55.3); credentials case is 'error [source]' with no location (TASK-71 CLI cases need build, R3). Existing fixture test/acceptance case with 'github:owner/repo@--upload-pack=touch' expects 'is not a valid git ref' text, which Rust words as 'stack[0].source must be a git ref: ...'; assert on the code and exit 1, not the TypeScript wording.

Lead decision applied: a null optional field or section counts as absent; a null required field is reported as missing. Done in the shared helper: Section::node (yaml/fields.rs) filters null, and text, whole_number, text_list, boolean and items all read through it, so it covers layer meta and the manifest. Exception forced by test 29 (account: null and {claude: null} are refused): new Section::written_node returns the node as written, used by layer/meta.rs for account and by account/declaration.rs for each per-target entry. Spec 5.1 and 5.2 each gained the sentence (5.1 names the account exception). Tests: layer/meta/tests.rs (an_optional_field_that_is_null_counts_as_absent, a_required_field_that_is_null_is_reported_as_missing, the_account_field_still_refuses_null_as_test_29_says) and stack/tests.rs (same first two names). rust:ci exit 0. Note: the shell was moved to a worktree path that holds only R2.8's files; this work is in the main checkout /Users/gautamr/dev-projects/agent-layerz.

Verified by the lead (2026-11-23): mise run rust:ci exits 0 with R2.8's emitter in the tree (212 tests); src/ and test/ unchanged. Lead decision: a null optional field counts as absent (as the reference does), a null required field is missing; account: null stays refused (test 29). The open choices written into §5.2 are accepted.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
The repo stack manifest parses into validated types: path:, github: and git: sources that print back byte for byte, git refs that refuse lists, numbers and unsafe values (fixes TASK-61), credentials in a source URL refused without printing them (TASK-71's parse rule), workspace paths, budgets, targets and options, with paths in name order. Null optional fields count as absent.
<!-- SECTION:FINAL_SUMMARY:END -->
