---
id: TASK-55.6.17
title: >-
  R5.17: Contributions carry their file and the layer's revision, and explain's
  action words
status: Done
assignee:
  - '@implementer'
created_date: '2026-11-23T18:30:00Z'
updated_date: '2026-11-23T23:14:00Z'
labels:
  - area:rust
  - area:merge
milestone: m-7
dependencies:
  - TASK-55.6.2
parent_task_id: TASK-55.6
priority: high
ordinal: 153390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Found by R5.1 (2026-11-24): Contribution in crates/alz-core/src/merge/provenance.rs holds only the layer and an Action (Define, Extend, Replace, Remove). Byte-identical headers need each contribution's layer file and the layer's revision or version (test 26 asserts a source like <layer>@<version>:layer.yaml), and explain needs per-kind action words (test 17 asserts e.g. 'extend: paths +[e2e/**]'). Extend the merge result so every contribution carries its ItemPath (the layer file) and the layer's LayerLabel input, and each kind's merge records the detail explain prints, as typed values (no strings built in merge). Pure alz-core; property tests that provenance stays in stack order; parity with the reference's explain words for the fixtures of tests 17 and 26.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Every contribution names its layer file and the layer's revision or version, in stack order
- [x] #2 Each kind's merge records the typed detail explain needs for the reference's action words (fixtures of tests 17 and 26)
- [x] #3 mise run rust:ci passes
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
R5.17 implementation (uncommitted, ACs not checked).
Design: merge/action.rs holds the typed Action (Define, Extend, Replace, Remove, Overlay{mode, changes: Vec<AgentChange>}, ExtendRule{paths}); AgentChange/AgentScalar/ListChange are typed values, no sentence built in merge. merge/provenance.rs: Origin{layer, revision: Option<Revision>, file: SafeRelPath} (Origin::new(&ResolvedLayer, file), Origin::of_item(layer, ItemPath)); Contribution holds an Origin and an Action (action() now returns &Action). stack::Revision plus ResolvedLayer::revision() hold the pin rev else version; LayerLabel::of now uses it. render/header.rs: LayerLabel::of_contribution and SourceRef::of_contribution (reuse of R5.2 types). render/action_word.rs: small pure Display formatter (ActionWord, AgentChange, ListChange) used to prove parity. item::Skill::file()/SkillDirectory::manifest() give the SKILL.md path. Every kind's merge (agents, rules, instructions, skills, hooks, mcp, settings, models, permissions, passthrough) passes an Origin; Merged fields stay private.
AC1: provenance/tests.rs property test provenance_follows_the_stack_and_names_files_of_the_layers (stack order, file exists in its layer, revision = layer version) plus unit tests for client@0.1.0:layer.yaml (test 26) and shop-api:layer.yaml without version.
AC2: rules/tests.rs the_contributions_of_a_rule_give_the_lines_of_alz_explain_of_case_17 builds the fixture's real files and gets '1. personal@1.0.0:rules/testing.md — rule' and '2. shop-api:rules/testing.md — extend: paths +[e2e/**]'; agents/tests.rs gives 'base', 'extend: skills +[domain-glossary]; body section', 'extend: body section' for case 03; existing merge tests updated to the typed actions.
AC3: mise run rust:ci exit 0 (fmt, clippy -D warnings, tests 1030 in alz-core lib, doc, msrv, deny, audit); mise run lint exit 0.
Open: (1) a skill's source is skills/<name>/SKILL.md (a real file); the reference prints skills/<name> (directory). Spec 4.3/10.3 should say which; R5.4 header. (2) Adjust with empty remove and add prints +[] (reference prints ops as written, so order of '+[..] -[..]' written together can differ: Rust normal form is -[..] +[..]). (3) The '(overridden)' suffix is derived by explain from a later Replace contribution for rule/skill/command/mcp, not stored. (4) Slot stubs have no contribution (SlotDemand); explain prints 'slot stub — stub' from it. (5) Instruction block sources keep one Traced per file, each contribution word 'block' is explain's.

R5.17 follow-up (lead decisions, byte parity with the reference; still uncommitted).
1. A skill is named by its folder: Skill::source() is skills/<name> (SkillDirectory::folder()), a command by its file commands/<name>.md, as sourceRef(layer, dir) and sourceRef(layer, path) in src/merge/skills.ts. Test skills/tests.rs a_skill_is_named_by_its_folder_and_a_command_by_its_file_as_the_reference_does gives personal@1.0.0:skills/arch and personal@1.0.0:commands/ship.md. The provenance proptest now includes a skill per layer and accepts a file or a folder that a layer file lies under.
2. List edits are kept as written: item::WrittenEdit (EditSegment{EditSign Add|Remove|Replace, items}) is read from the overlay field next to the folded ListEdit (AgentOverlay::tools_written/skills_written/targets_written, None when the overlay does not mention the list). AgentChange::{Skills,Targets,Tools} hold it; ListChange is gone. Display prints the segments in written order. I ran the reference describeOps through a scratch esbuild bundle: '+[a, b] -[c]' -> '+[a, b] -[c]', '-[c] +[a]' -> '-[c] +[a]', '[a, b]', '' -> 'skills ' (nothing after the word), '+[]' -> '+[]', '-[x] -[y] +[z]', YAML list [a, b] -> '[a, b]', "+['q', \"r\"]" -> '+[q, r]'. Test agents/tests.rs the_edits_of_a_list_are_shown_in_the_order_the_layer_wrote_them asserts 'extend: skills +[b, c] -[a]; targets [claude]' and 'extend: skills -[x] +[y] -[z]; targets ' (an empty string edit prints nothing, as in the reference).
Drift seen, not changed: the reference refuses a whitespace-only list string ('cannot parse'); the Rust reader accepts it as an edit with no segment (it prints like an empty string).
Checks: cargo clippy -D warnings clean, alz-core lib 1032 tests pass, mise run rust:ci exit 0, mise run lint exit 0.

Verified by the lead (2026-11-24): mise run rust:ci and mise run lint exit 0, src/ unchanged. Lead decisions applied for parity: a skill's source is its folder skills/<name> and a command's its file, as the reference prints; list edits keep the order the layer wrote them (WrittenEdit).
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Every contribution in the merged model now carries an Origin (layer, the pin's rev or the layer version, and the layer file or skill folder) and a typed Action whose detail explain needs (overlay changes as written, rule path extensions). Unit tests reproduce the reference's explain lines for tests 17 and 03 and the test 26 header source; a proptest keeps provenance in stack order.
<!-- SECTION:FINAL_SUMMARY:END -->
