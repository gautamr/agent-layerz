---
id: TASK-55.5.4
title: 'R4.4: Instructions and path-scoped rules'
status: Done
assignee:
  - '@implementer'
created_date: '2026-11-23T12:27:00Z'
updated_date: '2026-11-23T13:09:00Z'
labels:
  - area:rust
  - area:merge
milestone: m-7
dependencies:
  - TASK-55.5.3
parent_task_id: TASK-55.5
priority: high
type: task
ordinal: 117390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Plan for this step.
- item/instruction.rs and merge/instructions.rs (§7.1): blocks in stack order, then by file name in code-point order (§6), each tagged with its layer's kind and name. `mode: replace` and `mode: remove` in a more specific layer act on the more general block with the same file name. Unknown fields and other modes are `instructions` errors. targets/claude/CLAUDE.extra.md is collected per layer for R5. The instructions_max_lines budget counts the composed AGENTS.md and stays with R5's render.
- item/rule.rs and merge/rules.rs (§7.10): identity is the kebab-case file name; the most specific layer wins with the whole file; `mode: extend` appends its body under a layer heading and unions `paths`; paths entries are non-empty globs.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Test 02's case has a Rust test with the same verdict: three blocks in stack order, mode: remove drops a block and mode: replace replaces one (unit tests)
- [x] #2 Test 17's merge cases have Rust tests with the same verdict: a repo rule replaces the personal one, and an extend rule appends a section and unions paths (unit tests)
- [x] #3 Proptest: the rule merge is associative, path unions are idempotent, and block order depends only on stack order and code-point name order
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Implementer (2026-11-23), R4.4 done in the working tree (not committed, ACs not checked, per instruction).

Checks: mise run rust:ci exit 0 (fmt, clippy -D warnings, tests, doc, MSRV, deny, audit; alz-core lib 539 tests, was 502 before this task); mise run lint exit 0.

Files: alz-core item.rs, item/instruction.rs, item/rule.rs (per-layer readers, validated types RuleName, Glob, InstructionMode, RuleMode, InstructionEdit, RuleEdit, RuleBase::new refuses no globs), merge/instructions.rs, merge/rules.rs (RuleChain algebra, MergedRule), merge/testing.rs (test helpers for R4.5+), merge/stack.rs (Accumulator and Merged fields: instructions(), claude_extras(), rules()), merge/provenance.rs (Traced::filter_map), layer/heading.rs (LayerHeading '## <Kind title>: <name>') and Kind::title, yaml/fields.rs (Section::of_mapping), markdown.rs (Frontmatter::section), schema.rs (Refusal::Glob, InstructionMode, RuleMode), diagnostic.rs (Code::Instructions, Rule, Unresolved).

AC1 (test 02): merge::instructions::tests personal_client_and_repo_give_three_blocks_in_stack_order_and_remove_drops_one (reads the fixture files of 02 with include_str!), the_blocks_compose_the_text_of_the_expected_agents_md (composed blocks equal the text of fixtures/02-instructions/expected/AGENTS.md after its header comment), the_removed_block_is_gone_and_the_others_keep_who_made_them. Test 02 has one case; its CLAUDE.md half is R5's render. Extra rules covered: replace_puts_the_new_block_in_the_place_of_the_layer_that_replaced_it (provenance Define then Replace), a_file_without_mode_adds_a_block_even_when_a_more_general_layer_has_the_name, a_replace_or_remove_without_a_block_to_act_on_is_unresolved, an_unknown_field_and_an_unknown_mode_are_instructions_errors_that_name_the_file, a_block_that_is_not_closed_is_a_frontmatter_error_and_the_other_files_are_still_read, a_block_without_text_is_dropped_but_can_still_be_replaced, files_of_a_layer_are_in_code_point_order_of_the_whole_file_name, the_claude_text_of_each_layer_is_kept_in_stack_order_and_an_empty_one_is_not, crlf_files_give_the_same_blocks_as_lf_files.

AC2 (test 17): case 1 'a repo rules/backend.md replaces the personal one' -> merge::rules::tests::a_repo_rule_replaces_the_personal_one_whole (paths [server/**/*.ts], body, provenance personal Define + shop-api Replace). Case 2 'an extend rule appends a section and combines paths' -> an_extend_rule_appends_a_section_and_unions_the_paths (paths [**/*.test.ts, e2e/**], body with '## Repo: shop-api' section, provenance Define + Extend, the extension's own globs [e2e/**] for explain's 'paths +[e2e/**]'). Case 3 'a hand-written .claude/rules file is a collision' is a build case (R5), no merge verdict. Extra: an_extension_without_text_adds_its_paths_and_no_section, a_rule_defined_again_drops_the_extensions_before_it, two_extensions_keep_stack_order_and_each_glob_once, an_extension_with_no_rule_before_it_is_unresolved, a_rule_that_breaks_its_schema_is_a_rule_error_that_names_the_file, a_rule_whose_block_is_not_closed_is_a_frontmatter_error, rules_are_kept_by_name_in_code_point_order; item::rule::tests (name grammar, glob, mode).

AC3 (laws): proptests the_rule_merge_is_associative_and_has_an_identity, folding_a_stack_in_groups_equals_folding_it_whole, path_unions_are_idempotent (merge::rules::tests), block_order_depends_only_on_stack_order_and_name_order (merge::instructions::tests, listed in reverse and forward order), names_order_by_code_point and the kebab-grammar proptest (item::rule::tests).

Spec gaps decided from the TypeScript reference (spec is silent; need the architect): (1) instruction mode values: spec names only replace and remove; the reference also accepts 'append'; Rust accepts no explicit add/append (absent field = add), so 'mode: add' is an instructions error. The plan text says modes add/replace/remove. (2) replace or remove with no block to act on is an 'unresolved' error (reference behaviour); spec 10.1 lists unresolved only for overlay, extend rule, hook disable, alias. (3) a replace rule without paths is a 'rule' error 'paths is required; always-loaded text belongs in instructions/' (reference; spec 7.10 does not say). (4) blocks and Claude text are trimmed, and a block empty after trimming is dropped at the end (reference). (5) '## <Kind>: <name>' capitalises the first character of the kind label (test 02 expects it, spec silent); the kind is free text from layer.yaml, so R5 must print it as untrusted text. (6) a replacing instruction file moves its block to the replacing layer's place and heading (reference). Also: duplicate globs in one paths list are kept once (reference kept them on replace).

Lead decision (2026-11-23), point 1: instruction 'mode: append' is accepted and means add (InstructionMode::Append, read as InstructionEdit::Add); 'mode: add' stays refused, message now 'mode must be append, replace or remove'. Tests: item::instruction::tests::mode_append_adds_a_block_like_a_file_without_mode, the_modes_are_append_replace_and_remove_and_nothing_else, and merge::instructions::tests (an append file gives no error; add and prepend still do). mise run rust:ci exit 0 (alz-core lib 540 tests). Points 2-6 unchanged; U+FEFF trim difference is reference drift.

Verified by the lead (2026-11-23): mise run rust:ci exits 0, lint passes, src/ unchanged. Lead: mode: append accepted (the reference's default); points 2–6 accepted and collected for the spec on TASK-55.5.15.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Instructions (append, replace, remove; stack order then name order; per-layer CLAUDE.extra.md) and path-scoped rules (replace, extend with path unions) merge in alz-core with provenance, Rust tests with the verdicts of tests 02 and 17, and property tests for associativity, grouping and order.
<!-- SECTION:FINAL_SUMMARY:END -->
