---
id: TASK-55.5.9
title: 'R4.9: Permissions with the floor'
status: Done
assignee:
  - '@implementer'
created_date: '2026-11-23T12:27:00Z'
updated_date: '2026-11-23T14:14:00Z'
labels:
  - area:rust
  - area:merge
  - area:security
milestone: m-7
dependencies:
  - TASK-55.5.8
parent_task_id: TASK-55.5
priority: high
type: task
ordinal: 122390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Plan for this step (§7.6).
- item/permission.rs: allow, ask and deny as ListEdit of neutral rules (a plain YAML list adds; `+[..]`, `-[..]` and `[..]` strings edit, as the reference reads them); default_mode a PermissionMode ordered by strictness; floor: true needs a default_mode.
- merge: lists combine as OrderedSet. A more specific layer whose deny edit drops a rule of a more general layer is a `permission-floor` error naming the layer, the rule (stripped) and the layer that set it; the rule stays. default_mode: most specific wins, never looser than the strictest floor of a more general layer.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Test 07's cases have Rust tests with the same verdict: a repo layer removing a personal deny fails naming the layer and the rule, and allow, ask and deny combine (unit tests)
- [x] #2 Proptest: merging a further layer either refuses or keeps every deny rule of the stack, and the merged default_mode is never looser than a floor
- [x] #3 Unknown fields, an unknown mode and floor without default_mode are permissions errors (unit tests)
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
R4.9 implementation (not committed, ACs not checked).
Files: alz-core item/permission.rs (+permission/tests.rs), item/list_edit.rs (moved from item/agent/edit.rs; refine_edit takes from_list so a YAML list is Replace for overlays and an add for permissions), merge/permissions.rs (+permissions/tests.rs, permissions/laws.rs), Code::Permissions and Code::PermissionFloor in diagnostic.rs, Refusal::Permission in schema.rs, PermissionMode derives Ord (declaration order = strictness) and has as_str, Merged::permissions() -> MergedPermissions (allow/ask/deny as Traced rules, default_mode Traced, floor).
AC1: Test 07 cases -> merge/permissions/tests.rs: 'a repo layer removing a personal deny fails' -> case_07_a_repo_layer_removing_a_personal_deny_fails_naming_the_layer_and_the_rule (error [permission-floor] shop-api:permissions.yaml: deny rule 'read:.env*' is set by layer personal and cannot be removed by layer shop-api, no merged model); 'allow, ask and deny combine' -> case_07_allow_ask_and_deny_lists_combine_across_layers (allow [git status, mise run *], deny [read:.env*, rm -rf *, git push --force*], provenance personal/personal/shop-api). Both read the fixture files with include_str. Extra: replace [..] dropping a deny, every removed rule reported, owner is the layer that set it, add+remove in one layer ok, allow/ask removable, floor cases (looser refused, equal/stricter kept, floor binds only more specific layers, strictest floor and first declarer named, refused mode leaves state unchanged).
AC2: merge/permissions/laws.rs proptests: a_more_specific_layer_cannot_loosen_the_deny_rules_or_the_default_mode (kernel, typed files: deny of stack kept even on error, floor monotone, mode >= floor, error iff the layer tried to drop a deny or go looser than the floor); merging_a_further_layer_either_refuses_or_keeps_the_denies_and_the_floor (through YAML text and the whole merge; refusals only permission-floor; deterministic); a_file_read_from_its_text_is_the_file; unions: the_lists_combine_associatively / _idempotently / _hold_the_same_rules_in_either_order. Mutation-checked: dropping the keep-the-denies line or the floor check fails these tests.
AC3: item/permission/tests.rs: unknown field, unknown mode (default_mode), floor without default_mode (true or false), floor not a boolean, unknown tool, empty pattern, not a mapping, every problem reported; all code [permissions] with line:column.
Spec gaps and choices (spec unchanged): (1) §7.6 does not say that allow/ask/deny take edit strings (+[..], -[..], [..]) nor that a plain YAML list adds (unlike §7.2 overlays where a list replaces); implemented as the reference reads them. Allow and ask may drop rules, deny may not. (2) The permission modes and their order (bypass_permissions < auto < accept_edits < default < dont_ask < plan) are the reference's; the spec lists none (already noted for permission_mode in agents). (3) Rule grammar is the neutral tool grammar tool[:pattern]; the spec does not say that an empty pattern or an unknown tool is refused (§4.1 mentions an unknown permission tool only). (4) The reference warns permission-shadowed when a rule is in allow and deny; no such code in §10.1, so none is emitted (deny wins already in the targets). (5) floor: the spec says only 'floor: true'. A non-boolean floor is a permissions error (the reference ignores it); floor present without default_mode is an error also for floor: false, as in the reference. (6) A looser default_mode is refused and the layer's mode and floor are then ignored; a layer's own floor does not bind its own mode. (7) Diagnostics for the merge errors carry no line (as hooks), the file errors do.
Checks: mise run rust:ci exit 0 (fmt, clippy -D warnings, tests, doc, MSRV, deny, audit); mise run lint ok. test/ and src/ untouched, so mise run test not run.

Gap 4 resolved by the lead: Code::PermissionShadowed added. A merged allow rule that the merged deny list also holds gives 'warning [permission-shadowed] allow rule '<rule>' is also denied; deny wins' (reference wording, no place, as in the reference); the merge still succeeds. Test: a_rule_that_is_allowed_and_denied_is_a_warning_and_deny_wins in merge/permissions/tests.rs; the refusal property now looks at errors only. Spec pass must add permission-shadowed (warning) to 10.1. mise run rust:ci exit 0 after the change.

Verified by the lead (2026-11-23): mise run rust:ci exits 0, lint passes, src/ and test/ unchanged. The permission-shadowed warning kept (lead), its code goes into §10.1 in the spec pass.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Permissions merge in alz-core: allow, ask and deny as ordered unions with list edits, a deny no more specific layer can remove, and a default_mode floor that binds only more specific layers, so security only gets stricter down the stack (property tests that fail without the floor check); a rule both allowed and denied is a permission-shadowed warning; test 07 has Rust tests with the same verdicts.
<!-- SECTION:FINAL_SUMMARY:END -->
