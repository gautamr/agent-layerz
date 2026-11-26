---
id: TASK-110
title: Take native key rules and vocabularies from each adapter's descriptor
status: In Progress
assignee:
  - '@implementer'
created_date: '2026-11-25T19:06:00Z'
updated_date: '2026-11-26T07:57:00Z'
labels:
  - area:adapter
milestone: m-8
dependencies:
  - TASK-108
references:
  - decision-40
priority: medium
ordinal: 211390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
item/native_keys.rs holds Claude and Codex rows in the core. decision-40 point 3 moves them into descriptors (Codex as a descriptor without a renderer until its adapter exists); the core keeps the floor algebra, so the security floor stays core law.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 item/native_keys.rs holds no Claude or Codex row and still holds Strictness and loosenings (grep)
- [ ] #2 settings.yaml validates a target section with that target's descriptor, and the existing settings tests pass unchanged
- [ ] #3 The laws of merge/settings/laws.rs pass for a test descriptor with a flag floor and a ladder floor (proptest)
- [ ] #4 The acceptance suite and mise run alz:check pass with unchanged output
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Implementation (commit 3fb18da). Descriptor shape: NativeVocabulary keeps first_class_keys, reserved_keys, floor_keys; FloorRule is now TrueIsStricter | FalseIsStricter | Ranked(Vec<Vec<String>>) | NoAdditions | Union (the first cut lacked the last two and flattened the ranks); NativeVocabulary::with_key_rules builds it (non_exhaustive). Claude rows: crates/alz-core/src/target/claude/key_rules.rs, wired in claude/descriptor.rs. Codex rows: descriptor-only entry crates/alz-core/src/target/codex.rs, held by Registry (PLANNED map) and handed out by Registry::key_rules(target) when no adapter is registered; the 'not implemented yet' message keys off the same map. settings.yaml: SettingsItems::read(layer, &Registry) -> registry.key_rules(target); LayerContext carries the registry; merge() and check_alone() take &Registry; AdoptionInput has a registry field. A target with neither (e.g. antigravity, or claude in an empty registry) is read as a mapping with no diagnostic. AC1: grep -inE 'claude|codex' item/native_keys.rs returns nothing; Strictness, Looseness, KeyRules::loosenings remain there. AC2: item/settings/tests.rs tests added a_target_section_is_checked_with_the_key_rules_of_its_descriptor, a_target_with_neither_an_adapter_nor_a_descriptor_is_read_as_a_mapping_and_nothing_more, codex_is_validated_without_an_adapter_by_the_descriptor_only_entry_of_the_core; existing tests' assertions are unchanged (only the call gained a &Registry argument, and the test that iterated KEYS iterates the registry's rows). AC3: merge/settings/laws.rs::a_flag_floor_and_a_ladder_floor_of_a_test_descriptor_never_loosen_down_the_stack (proptest, test adapter via merge/testing.rs::registry_with_vocabulary); the existing laws run on the built-in registry. The spec lists oracle (native_keys/spec_lists.rs) now compares the descriptors' rows with the spec lists. AC4: mise run rust:ci exit 0 (fmt, clippy -D warnings, 79 test binaries ok, doc, msrv, deny, audit); mise run alz:check: check passed, 67 generated files match; golden trees and fixtures unchanged. docs/spec-coverage.md: Output checks row now covered (summary updated); guard passes. Left open: AGENT_KEYS (agent native fields refused keys) and neutral_keys/CLAUDE_AGENT_FIELDS/CLAUDE_SETTINGS typed tables are still core tables (AGENT_KEYS moved out of native_keys.rs into item/agent/native.rs), because reading agents has no registry yet; they belong with the descriptor's agent_fields/settings_keys.
<!-- SECTION:NOTES:END -->
