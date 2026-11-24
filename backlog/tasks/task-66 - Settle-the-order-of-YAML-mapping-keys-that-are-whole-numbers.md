---
id: TASK-66
title: Settle the order of YAML mapping keys that are whole numbers
status: Done
assignee: []
created_date: '2026-11-23T03:51:00Z'
updated_date: '2026-11-24T05:55:00Z'
labels:
  - discovered
  - area:spec
  - area:rust
milestone: m-7
dependencies: []
references:
  - docs/LAYERING-SPEC.md
  - src/core/yaml.ts
  - src/adapters/claude/agents.ts
priority: medium
type: bug
ordinal: 83390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Found while planning TASK-64. A JavaScript object always lists keys that are whole numbers ("0", "9", "10", not "007") first, in numeric order, whatever their insertion order. The TypeScript reference reads layer YAML into plain objects (src/core/yaml.ts) and writes frontmatter from them, so source order is not kept for such keys: a native field under an agent's or skill's claude: map that is a whole number is written before name: (src/adapters/claude/agents.ts Object.assign, then stringifyYaml), and mcp.yaml or settings.yaml entries reach merge Maps and listings (alz ls, alz explain) in that hoisted order. A Rust port that keeps source order (an ordered map) would not be byte-identical. TASK-64 fixes only sorted output (stableJson); this task decides insertion-order output. Out of scope: sorted JSON keys (TASK-64).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The spec states the key order of generated frontmatter and of mapping entries read from layer YAML, including keys that are whole numbers
- [x] #2 The Rust alz follows it; an acceptance test with native frontmatter keys 10 and 9 after the fixed fields checks the generated agent file, marked reference drift where the TypeScript reference fails it
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Owner (2026-11-23, decision-30): src/ is never changed; this bug is fixed only in the Rust alz, in R5 (TASK-55.6, which depends on this task); AC #2 reworded from a TypeScript fix. The spec part (AC #1) can be written before R5.

Rust decision (lead, applied in R4.5, TASK-55.5.5): the native fields of an agent (claude:/codex: maps) keep source order. A key keeps the place of its first appearance in the stack, a key a later layer adds comes after the existing ones, a later value replaces or deep-merges in place without moving the key, and whole-number keys are ordinary keys in source order (the reference hoists them: drift). Implemented as alz-core NativeMap/NativeValue (item/agent/ordered_tree.rs). Settings stay sorted by code point because their JSON output sorts keys (spec section 6), so Tree is unchanged. R5 writes the native fields in the order NativeMap holds them; the spec text for AC #1 still needs the architect.

Architect (2026-11-23, R4 spec pass, on feat/rust-r4-task-55.5, not committed): AC #1 is met in docs/LAYERING-SPEC.md.
- §7.2 "Key order of native fields" states the rule:
  - native fields keep source order: a key keeps the place of its first appearance in stack order, a key a later layer adds comes after the existing ones, and a later value replaces or merges in place;
  - a whole-number key (9, 10) is an ordinary key and keeps its place;
  - a generated agent file writes the neutral fields first, then the native fields in that order;
  - the reference hoists whole-number keys, which is marked reference drift (TASK-66);
  - settings are sorted by key.
- §10.2 "Values": mapping entries read from layer YAML keep source order only for an agent's native fields. Everywhere else they are sorted by key in code-point order, settings and MCP servers included, and a whole-number key is ordered like any other.
Evidence that the code matches:
- crates/alz-core/src/item/agent/ordered_tree.rs, whose module docs give the same rule;
- the tests native_keys_keep_source_order_and_a_whole_number_key_is_not_hoisted and a_key_a_later_layer_adds_comes_after_the_existing_ones_and_a_replaced_value_stays_in_place in crates/alz-core/src/merge/agents/tests.rs (R4.5).
mise run lint exit 0. AC #2 (the generated agent file and its acceptance test) is R5's.

AC #2 evidence (R5.4, TASK-55.6.4, uncommitted): Rust alz-core target/claude renders native fields in NativeMap order after the fixed fields (unit test native_fields_hold_the_keys_10_and_9_in_source_order_after_the_fixed_fields, output name, description, "10", "9", effort); acceptance case in test/acceptance/18-native-fields.test.ts (fixture 18-native-fields/repo-key-order) marked referenceDrift TASK-66, expected-fail against the reference (it writes "9", "10" before name), must pass against ALZ_BIN once alz build exists (R5.8). AC not checked.

R5 (TASK-79 merged): agent keys under claude: are typed with fixed names, so a whole-number key cannot reach an agent's claude: map any more (only codex: is untyped). Acceptance case 18 'repo-key-order' now uses maxTurns, effort, color in non-alphabetical source order; the reference keeps named keys in source order, so it passes against the reference and is a plain it(), no longer referenceDrift. Whole-number key ordering is still shown in Rust by the settings test (modelOverrides map, code-point order). AC #2 as worded (agent keys 10 and 9) is no longer expressible for claude:; spec 7.2 says only an untyped map (codex:) can hold such a key. Left for the architect: decide whether AC #2 is met or needs a codex: case.

Lead decision (2026-11-24): TASK-79 refuses agent claude: fields that are not recorded, so whole-number keys can no longer appear there. AC #2 is met for Claude by test 18's source-order case (maxTurns, effort, color after the fixed fields), which the reference passes too (no drift); it is checked against the Rust binary once alz build exists (R5.8). Whole-number keys stay possible only under codex:, which the Codex adapter renders (TASK-18 carries that case).

Verified by the lead (2026-11-24): test 18's source-order case (accepted fields maxTurns, effort, color after the fixed fields; keys 10 and 9 replaced per the lead decision after TASK-79) passes against ALZ_BIN=target/release/alz and against the reference, so it is no drift. Whole-number keys under codex: are TASK-18's.
<!-- SECTION:NOTES:END -->
