---
id: TASK-55.3.5
title: >-
  R2.5: Layer meta, commit identity and the account declaration as validated
  types
status: Done
assignee:
  - '@implementer'
created_date: '2026-11-23T05:18:00Z'
updated_date: '2026-11-23T06:13:00Z'
labels:
  - area:rust
  - area:model
milestone: m-7
dependencies:
  - TASK-55.3.4
parent_task_id: TASK-55.3
priority: high
type: task
ordinal: 94390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Plan for this step. Spec §5.1, §7.12, §7.13 "Account names" and R2.1. Modules `crates/alz-core/src/layer.rs` with `layer/{name,kind,visibility,meta}.rs`, `identity.rs` with `identity/commit_identity.rs`, `account.rs` with `account/{name,declaration,protected}.rs`, and `target.rs` holding only `TargetName { Claude, Codex }` (the sealed adapter trait comes in R5).

Types:
- `LayerName` (the spec pattern from R2.1), `Kind { Client, Other(KindLabel) }` (only `client` has a meaning, §7.12), `Visibility { Public, Private, Confidential }` with derived `Ord` as the privacy order, `LayerVersion` and `Description` (YAML text only), `SlotName`, `ForbidPatternText` (kept as text: compiling it is the leak guard's job in R5, with its own decision on a regex crate).
- `CommitIdentity { name: IdentityName, email: EmailAddress }` with §7.12's rules.
- `AccountName` (`^[a-z0-9][a-z0-9-]*$`, at most 64), `AccountChoice { Default, Named(AccountName) }` (`default` always means the default account), `AccountDeclaration { Every(AccountChoice), PerTarget(NonEmptyTargetMap<AccountChoice>) }`.
- `LayerMeta` with each field; `LayerMeta::from_yaml(&yaml::Node) -> Result<LayerMeta, SchemaErrors>` collects every field error so lint reports all of them, and refuses a file with `stack` as a stack manifest.
- `account/protected.rs`: the pure Codex pattern match and protected names of §7.9, which R1.3 moved to reference with "Rust unit test in the account model (R2)": the reference's 15 cases become a table test.

Laws (proptest): `AccountName::parse(n.as_str()) == Ok(n)` and the same for `LayerName`; a generator of strings the rule refuses never yields a value; no schema error contains the refused value.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Each case of test 29's 'account layer field: schema' table and test 26's commit_identity tables has a Rust unit test with the same verdict, and no refused value appears in any error
- [x] #2 LayerMeta::from_yaml reports unknown fields as quoted keys, a number in a text field (version: 1.10) with the quoting hint, and every field error of one file at once (unit tests)
- [x] #3 Round-trip and refusal property tests for AccountName and LayerName pass, and the 15 Codex pattern cases pass
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Implementation notes (implementer). Modules added in alz-core: target.rs (TargetName), layer.rs + layer/{name,kind,visibility,text,meta}.rs, identity.rs + identity/commit_identity.rs, account.rs + account/{name,declaration,protected}.rs, schema.rs (SchemaError, SchemaErrors non-empty and in document order, Refusal, zip/zip3, refine/require/unknown_fields). Shared-file edits kept small: lib.rs (5 pub mod lines), diagnostic.rs (sealed mod pub(crate) so the name types implement Printable), untrusted_text.rs (is_control/is_format pub(crate)), yaml/fields.rs (Section::at, path, field_path, node, key_position).
AC1 evidence: layer::meta::tests: account_accepts_what_test_29_accepts, account_per_target_*, account_refuses_what_test_29_refuses_and_prints_no_value (all 20 refused rows; asserts no refused value in the printed diagnostic), a_commit_identity_that_test_26_refuses_is_refused_and_no_value_is_printed (23 name rows + 5 email rows; rows with a decoded control character are refused by yaml::parse first, and the type refuses them too), a_commit_identity_value_that_is_not_text_says_to_quote_it, a_commit_identity_has_exactly_the_keys_name_and_email, plus type-level tables in identity::commit_identity::tests; proptests no_schema_error_holds_a_refused_{account_value,layer_name_or_kind_or_visibility,identity_value}.
AC2 evidence: unknown_fields_are_reported_as_quoted_keys_and_never_with_their_values, an_unknown_key_is_stripped_and_cut_like_every_quoted_key, a_number_in_a_text_field_gets_the_quoting_hint (version: 1.10), every_field_error_of_one_file_is_reported_at_once_in_document_order (10 errors with positions), required_fields_that_are_missing_are_reported_together, a_stack_manifest_is_not_a_layer.
AC3 evidence: proptests in layer::name and account::name (round trip, refused alphabet, edge/double separators, 65+ chars); account::protected::tests::the_codex_pattern_cases_of_the_reference (15 cases). 
Checks: mise run rust:ci exit 0 (fmt, clippy -D warnings, tests 130 unit + doc tests, doc, MSRV, deny, audit). TS mise run lint/test not run: no TypeScript changed.
Open for R2.11: test 29's schema table asserts stderr contains "'account" and test 26 asserts "commit_identity"; the spec (§10.1) prints a key path without quotes (account, commit_identity.name), so test 29's assertion must become referenceDrift or be loosened to "account". A refused unknown account target now reads "unknown field 'cursor' in account" instead of the reference's "'account' keys must be target names". LayerVersion/Description/SlotName/ForbidPatternText are unvalidated text newtypes (spec states no rule); decoded control characters are already refused by yaml::parse.

Verified by the lead (2026-11-23): mise run rust:ci exits 0 (160 tests); src/ and test/ unchanged.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Layer meta, commit identity and the account declaration are validated types in alz-core (LayerName, Kind, Visibility, IdentityName, EmailAddress, AccountName, AccountDeclaration, TargetName, the protected variable names), parsed from the YAML tree with every field error of a file reported at once in document order, unknown fields as quoted keys and never a value. Tables from tests 26 and 29 and property tests cover them.
<!-- SECTION:FINAL_SUMMARY:END -->
