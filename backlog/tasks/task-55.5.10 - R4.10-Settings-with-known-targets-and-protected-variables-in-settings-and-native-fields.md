---
id: TASK-55.5.10
title: >-
  R4.10: Settings with known targets, and protected variables in settings and
  native fields
status: Done
assignee:
  - '@implementer'
created_date: '2026-11-23T12:27:00Z'
updated_date: '2026-11-23T14:30:00Z'
labels:
  - area:rust
  - area:merge
  - area:security
milestone: m-7
dependencies:
  - TASK-55.5.9
  - TASK-55.5.1
parent_task_id: TASK-55.5
priority: high
type: task
ordinal: 123390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Plan for this step (§7.9), the known-target rule of TASK-55.5 AC #3.
- item/settings.rs: settings.yaml parses into an ordered map of TargetName to SettingsTree (alz's ordered tree). A top-level key that is not a known target is a `settings` error naming the layer and the file, whatever the key, and that layer's settings are not merged; each target's value must be a mapping. Keys that duplicate a first-class item are refused per target (claude: permissions, hooks, model, mcpServers, enabledMcpjsonServers, disabledMcpjsonServers, enableAllProjectMcpServers, disableAllHooks; codex: model, mcp_servers, hooks, approval_policy, sandbox_mode).
- merge: DeepMerge per target (R4.3, owner answer 5): the left fold, a later kind replaces, list items deduplicated by structural equality.
- Protected variables (account/protected.rs): the keys of claude.env; shell_environment_policy anywhere under codex, profiles included (set keys, exclude entries, filters not set to include, with the Codex pattern match); the native agent fields (claude.mcpServers env in both shapes, codex.mcp_servers env, codex shell_environment_policy). Each is a `protected-variable` error naming the layer, the file and a key path chosen by alz, never the text; names must be plain names and compare ignoring ASCII case; a list or map of the wrong type is refused.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 A settings.yaml whose top-level key is not a known target is refused with a settings error naming the file, whatever the key (table test of keys), and the merged settings are an ordered map keyed by TargetName
- [x] #2 The reserved keys of each target are refused naming the file that sets them, and the deep merge follows §7.9: a later kind replaces and list items equal as ordered trees appear once (unit tests), associative over kind-consistent trees (proptest)
- [x] #3 Every protected-variable case of §7.9 for settings.yaml and native agent fields is refused with code protected-variable, names no value, and matches names in any ASCII case (unit tests and a proptest over case variants)
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Implementer (R4.10), evidence per criterion. Checks run: mise run rust:ci (fmt, clippy -D warnings, 773 alz-core tests plus the other crates, doc, MSRV, deny, audit) exit 0; mise run lint exit 0.
AC1 (known targets only): item/settings.rs reads settings.yaml into Settings = BTreeMap<TargetName, SettingsTree>; any other top-level key is a settings error 'unknown field <key>' naming layer:file:line:col, the file gives no item, so none of its settings reach the fold. Tests: item::settings::tests::a_top_level_key_that_is_not_a_known_target_is_refused_naming_the_file_whatever_the_key (table of 17 keys x 3 bodies), a_top_level_key_is_accepted_exactly_when_it_is_a_target (proptest), every_unknown_key_is_reported_and_one_that_is_not_text_is_named_by_its_kind, a_file_that_is_not_a_mapping_and_a_target_that_is_not_a_mapping_are_refused, no_settings_of_a_file_are_read_when_any_top_level_key_is_refused; merge::settings::tests::a_layer_with_a_refused_top_level_key_is_reported_and_nothing_is_merged and the_merged_settings_are_an_ordered_map_of_the_targets_that_some_layer_set. Merged::settings() is BTreeMap<TargetName, Traced<SettingsTree>>.
AC2 (reserved keys and the deep merge): item::settings::tests::a_key_that_duplicates_a_first_class_item_is_refused_per_target_naming_the_file_and_the_key (all 8 claude and 5 codex keys), every_reserved_key_of_a_file_is_reported_and_the_other_target_is_not_confused, a_reserved_name_below_the_top_of_a_target_is_an_ordinary_key. Merge: merge::settings::tests (key by key, later scalar wins, lists concatenated without repeats with mappings in another key order as one item, later kind replaces, provenance Define then Extend, determinism). Laws, merge::settings::laws: associative over kind-consistent trees, identity, idempotent over de-duplicated lists, the fold equals the per-target deep merge.
AC3 (protected variables): item/protected_variable.rs (checks over the sorted Tree), item/env_var_name.rs (EnvVarName grammar, protected() in any ASCII case, fixed spelling printed), account::protected_variable. Tests: item::protected_variable::tests (every form x every protected variable: claude.env, codex set, lower case, exclude exact, ? wildcard, * lower-case, *home* style, filters not include, profile; plain-name refusals; wrong-type refusals; allowed controls; both mcp server shapes; proptest over case variants asserting the fixed name is printed and the written text is not), item::settings::tests (through the file reader, code protected-variable, no item), item::agent::tests (claude.mcpServers both shapes, codex.mcp_servers, codex shell_environment_policy, in a base agent and in an overlay, all six variables; case; plain-name and wrong-type), merge::settings::tests::a_protected_variable_is_refused_naming_the_layer_and_the_file.
Case-to-test mapping: the task text names tests 11, 26 and 29, but none of their cases concerns settings or protected variables. The cases that do are in test/acceptance/24-untrusted-repo.test.ts (fixture 29-account-layer): FORMS x PROTECTED_VARIABLES for settings.yaml and agent frontmatter -> item::protected_variable::tests::every_form_of_every_protected_variable_is_refused_naming_the_variable and item::agent::tests::a_native_field_that_sets_a_protected_variable_is_refused_in_an_agent_and_in_an_overlay; 'names the key path and the variable, never a lower-case spelling' -> a_lower_case_spelling_is_named_by_the_fixed_name_and_never_printed; 'lint of a single layer reports it too' -> item::settings::tests (R4.15 wires lint); 'cannot be checked' rows -> a_list_or_map_of_the_wrong_type_cannot_be_checked_and_is_refused; 'still builds' rows -> what_the_policy_allows_is_not_refused and merge::settings::tests::settings_that_set_other_variables_and_names_that_are_plain_keys_merge; 'not a plain name' rows -> a_name_that_is_not_a_plain_name_is_refused_without_printing_it (a NUL key is refused earlier, by the YAML reader); 'filters action misspelt' -> a_filters_action_that_is_not_exactly_include_does_not_exempt_the_pattern; the 'plain keys nested under a target' case -> the same merge::settings::tests case as 'still builds'. Passthrough and mcp.yaml rows are R4.11 and R4.12.
Spec gaps and choices (spec not edited): (1) the spec does not say whether a target value of null (claude: with nothing) is an error; read as 'each value is a mapping', so it is a settings error, as in the reference. (2) A shell_environment_policy that is not a mapping, and claude.mcpServers / codex.mcp_servers of a type other than map or list, are not refused (as the reference); the spec's 'wrong type is refused' covers env, set and pattern lists only. A stricter reading would need a spec line. (3) protected-variable diagnostics are placed at the file without line and column, as the key paths come from a sorted tree; the reserved-key and unknown-key errors carry line:column. (4) A pattern that matches several protected variables gives one error per variable.

Implementer follow-up (lead decision, stricter than the reference): a shell_environment_policy under codex that is not a mapping (null is nothing to check) is a settings error in settings.yaml and an agent error in native fields, naming codex.shell_environment_policy; claude.mcpServers that is not a mapping or a list (the list shape is valid, §7.9) and codex.mcp_servers that is not a mapping are agent errors naming the key (mcp keys in settings.yaml are reserved keys and refused already). Tests: item::protected_variable::tests (a_policy_that_is_not_a_mapping_is_an_error_of_the_file_kind_naming_the_key, an_mcp_servers_value_of_another_type_is_an_error_naming_the_key), item::settings::tests, item::agent::tests. This replaces gap 2 of the earlier note; the spec needs a line for it (§7.9: a policy or server field of another type is refused). mise run rust:ci and mise run lint exit 0.

Verified by the lead (2026-11-23): mise run rust:ci exits 0, lint passes, src/ and test/ unchanged; wrong-typed shell_environment_policy and MCP server values are refused (lead), a list for claude.mcpServers stays valid per §7.9.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Settings merge in alz-core as an ordered map of the known targets to settings trees (any other top-level key is a settings error naming the file, whatever the key, and that file is not merged), with the owner's left-fold deep merge and its laws as property tests; keys that duplicate first-class items are refused per target; protected variables are refused in settings and native fields in every §7.9 form and case, and wrong-typed values that could hold environment settings are refused.
<!-- SECTION:FINAL_SUMMARY:END -->
