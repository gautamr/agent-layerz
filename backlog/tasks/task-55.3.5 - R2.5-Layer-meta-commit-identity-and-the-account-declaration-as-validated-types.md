---
id: TASK-55.3.5
title: >-
  R2.5: Layer meta, commit identity and the account declaration as validated
  types
status: In Progress
assignee:
  - '@implementer'
created_date: '2026-11-23T05:18:00Z'
updated_date: '2026-11-23T06:04:00Z'
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
- [ ] #1 Each case of test 29's 'account layer field: schema' table and test 26's commit_identity tables has a Rust unit test with the same verdict, and no refused value appears in any error
- [ ] #2 LayerMeta::from_yaml reports unknown fields as quoted keys, a number in a text field (version: 1.10) with the quoting hint, and every field error of one file at once (unit tests)
- [ ] #3 Round-trip and refusal property tests for AccountName and LayerName pass, and the 15 Codex pattern cases pass
<!-- AC:END -->
