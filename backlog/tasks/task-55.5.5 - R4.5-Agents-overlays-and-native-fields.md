---
id: TASK-55.5.5
title: 'R4.5: Agents, overlays and native fields'
status: Done
assignee:
  - '@implementer'
created_date: '2026-11-23T12:27:00Z'
updated_date: '2026-11-23T13:26:00Z'
labels:
  - area:rust
  - area:merge
milestone: m-7
dependencies:
  - TASK-55.5.4
parent_task_id: TASK-55.5
priority: high
type: task
ordinal: 118390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Plan for this step (§7.2). Tool narrowing is R4.6.
- item/agent.rs: Agent (kebab-case name equal to the file stem, description required, role, model, tools, skills, isolation, permission_mode, memory, targets); Overlay (extends equal to the file stem, OverlayMode Extend or Replace, `~` keeps the inherited value, ListEdit for skills and tools, allow_widen); NativeFields, an ordered map of TargetName to tree, which refuses a key that has a neutral field.
- merge/agents.rs: scalars Last; lists by ListEdit in stack order, deduplicated with order kept; extend appends one body section per layer under "## <layer kind> context: <layer name>"; replace replaces the body with an agent-replace warning; replace with no base defines a new agent; an overlay without a base is `unresolved`; two base definitions of one name are an error; native fields DeepMerge.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Test 03's merge case has a Rust test with the same verdict: base, skills overlay and body overlay give one agent with the merged skills, three body sections and three provenance entries (unit test)
- [x] #2 Test 18's cases have Rust tests with the same verdict: claude fields pass through and overlays deep-merge them, and a claude key with a neutral field is refused naming the key (unit tests)
- [x] #3 Two base agents with one name, an overlay without a base and an extends that differs from the file name are errors, replace warns, and ListEdit application is associative (unit tests and proptest)
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
R4.5 implementation notes (implementer). Code changed only; nothing committed, no AC checked.

Files: crates/alz-core/src/item/agent.rs, item/agent/{vocabulary,native,edit,tests}.rs, item/kebab_case.rs (shared kebab grammar, RuleName now uses it), item.rs (exports, zip_diagnostics), merge/agents.rs + merge/agents/tests.rs, merge.rs, merge/stack.rs (agents field in Accumulator and Merged, Merged::agents()), diagnostic.rs (Code::Agent, AgentDuplicate, AgentOverlay, AgentReplace), schema.rs (Refusal::Agent(AgentRefusal)).

Design: AgentBase / AgentOverlay (item) are read per file; AgentScalars uses Last; lists use OrderedSet and ListEdit (overlay text parsed to one ListEdit by folding the segments with ListEdit::merge); NativeFields = BTreeMap<TargetName, Tree> merged with Tree::deep_merge; MergedAgent with AgentBody (text + one ContextSection per extending layer, compose() joins with a blank line); merged agents are Traced (Define, Extend, Replace). Bases of a layer apply before its overlays.

AC1 evidence (merge/agents/tests.rs): a_base_a_skills_overlay_and_a_body_overlay_give_one_agent, the_agent_lists_its_three_sources_in_stack_order (personal Define, acme Extend, shop-api Extend), the_body_has_one_section_per_layer_under_the_layer_kind_and_name. Fixtures are include_str!'d from test/fixtures/03-agent-overlay.
AC2 evidence: claude_fields_pass_through_and_overlays_deep_merge_them, a_claude_key_with_a_neutral_field_is_refused_naming_the_key (exact line: error [agent] shop-api:agents/reviewer.md:5:3: 'claude.tools' has a neutral field; ...), plus every_neutral_key_..._reported, an_overlay_may_not_put_a_neutral_key_in_a_target_map_either, native_maps_deep_merge_nested..., a_whole_number_key_..._not_hoisted.
AC3 evidence: two_base_agents_with_one_name_are_an_error_naming_the_first_layer (agent-duplicate), an_overlay_without_a_base_is_unresolved, an_overlay_whose_extends_differs_from_its_file_name_is_an_error (agent-overlay), a_replace_overlay_replaces_the_whole_body_and_warns (agent-replace warning, merge still succeeds), a_replace_overlay_with_no_base_defines_a_new_agent_without_a_warning; ListEdit laws in item/agent/tests.rs: writing_edits_one_after_the_other_applies_them_one_after_the_other, composing_the_edits_of_a_stack_is_associative_and_applying_the_composition_is_applying_each (proptest, through the text parser), no_text_makes_the_segment_reader_panic.

Case-to-test mapping, test 03 (acceptance 03-agent-overlay.test.ts): case 1 "base + skills overlay + body overlay give one merged agent" (merge part: skills, three body sections, tools, role; the rendered file and the write:docs/** warning are R5) -> a_base_a_skills_overlay_and_a_body_overlay_give_one_agent + the_body_has_one_section_per_layer_under_the_layer_kind_and_name (compares with the fixture's expected file). Case 2 "explain agent architect lists all three sources in order" (merge part: the provenance order) -> the_agent_lists_its_three_sources_in_stack_order; the explain text and domain-glossary in the output are R5. Test 18: case 1 "passes claude: fields through, and overlays deep-merge them" -> claude_fields_pass_through_and_overlays_deep_merge_them (effort high, maxTurns 60, color blue; the output not containing "claude:" is R5). Case 2 "rejects a claude: field that has a neutral equivalent" -> a_claude_key_with_a_neutral_field_is_refused_naming_the_key.

Checks: mise run rust:ci exit 0 (fmt, clippy -D warnings, 577 alz-core lib tests plus the other crates, doc, MSRV, deny, audit); mise run lint exit 0. Acceptance suite not run (needs the Rust binary: R5).

SPEC GAPS / choices for the architect pass:
1. Native-field key order (TASK-66). Tree::Map is a BTreeMap, so native fields are held in code point key order, not source order, and a whole-number key is not hoisted but sorts as text ("10" before "9"). If the generated agent frontmatter must keep source order, R5 needs an order-preserving tree (Tree's equality ignores key order by design, see §7.9 dedup), or the spec must say sorted. Not changed here (R4.3's type).
2. §7.2 does not list the keys with a neutral field inside a target map ("tools, model, permissionMode, ..."). I used the reference's lists: claude = name, description, tools, model, permissionMode, skills, isolation, memory; codex = name, description, model, developer_instructions. Neutral spellings such as permission_mode or role are not refused (as the reference).
3. memory: spec lists none | project | user; the reference also accepts local. Accepted (Memory::Local); one line to remove if the spec is right.
4. permission_mode values are not listed in §7.2/§7.6; used the reference's: bypass_permissions, auto, accept_edits, default, dont_ask, plan.
5. §10.1 `agent` code lists "too many preloaded skills", but no limit is defined (always_loaded_skills_max is the main-session budget, the reference never checks it in the agent merge). Not implemented.
6. Which code an overlay's other schema errors get: the table gives agent-overlay only for extends and mode; I gave every other overlay field error (unknown field, bad scalar or list item) the code agent. The reference uses agent-overlay for all. Also: missing `extends` is agent-overlay "extends is required" (the reference: a mismatch message).
7. Overlay `targets` edits (+/-/[]) are not in the spec's overlay example; the reference accepts them, kept.
8. `-[..]` of a tool on an agent with no tools list (inherits all) is an error agent-overlay as in the reference; the spec is silent. `+[..]` or a replace on such an agent gives it that list; the widening rule for it and allow_widen in a public layer (tool-widen) are R4.6. The unknown-tool check is R4.6 too (ToolEntry is only a non-blank text).
9. List-edit syntax detail is not in the spec: items split at commas, trimmed, one pair of matching quotes removed, empty items dropped, no `]` inside an item; segments may be in any order and repeat; empty text = no change. Reference behaviour.
10. §7.2 says name must be "required, kebab-case"; "name equals the file stem" is only implied by §4.1 (the reference errors: code agent, "name must match the file name", no location of the value). A replace overlay with no base and no description is code agent ("description is required to define a new agent").
11. Native maps: a YAML key that is not text (1, true, null) becomes its text; `1` and `"1"` in one map collide and the later wins. Protected-variable checks of native mcpServers/policies are R4.10.
12. Missing-field errors are located at 1:1 (the section start convention of earlier readers), e.g. "agents/a.md:1:1: description is required".

UPDATE (lead decision, TASK-66): native fields keep source order. This replaces gap 1 and the Tree-based NativeFields above. New item/agent/ordered_tree.rs: NativeMap (a Vec of unique (key, NativeValue), alz-owned, no new crate) and NativeValue; NativeFields is now BTreeMap<TargetName, NativeMap>. Tree and settings are unchanged (sorted). Rule: a key keeps the place of its first appearance in the stack; a key a later layer adds comes after the existing ones; a later value replaces or deep-merges in place without moving the key; whole-number keys are ordinary keys in source order; lists are concatenated and de-duplicated with items compared as sorted trees (mappings equal in another key order are one item). A key that comes twice in one file (1 and "1") keeps the first place and the last value.
Tests (merge/agents/tests.rs): native_keys_keep_source_order_and_a_whole_number_key_is_not_hoisted, a_key_a_later_layer_adds_comes_after_the_existing_ones_and_a_replaced_value_stays_in_place, list_items_that_are_mappings_equal_in_another_key_order_are_one_item, plus the test 18 and nested deep-merge cases now asserting order. Proptests (ordered_tree.rs): deep merge associative in value and key order, a key keeps its first place. mise run rust:ci exit 0, mise run lint exit 0.

Verified by the lead (2026-11-23): mise run rust:ci exits 0, lint passes, src/ unchanged. Lead decision TASK-66: native fields keep source order (NativeMap), settings stay sorted; the other choices accepted and collected for the spec on TASK-55.5.15.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Agents merge in alz-core: bases, overlays that extend (list edits +[..] -[..] [..] with their laws as property tests) or replace (with a warning), per-target native fields that deep-merge in source order (TASK-66) and refuse keys that have a neutral field, and errors for duplicate agents, overlays without a base and a mismatched extends; tests 03 and 18 have Rust tests with the same verdicts.
<!-- SECTION:FINAL_SUMMARY:END -->
