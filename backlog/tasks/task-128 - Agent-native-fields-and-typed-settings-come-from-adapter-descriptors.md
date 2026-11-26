---
id: TASK-128
title: Agent native fields and typed settings come from adapter descriptors
status: In Progress
assignee: []
created_date: '2026-11-26T07:58:00Z'
updated_date: '2026-11-26T09:04:00Z'
labels:
  - area:core
  - refactor
milestone: m-8
dependencies:
  - TASK-110
priority: medium
ordinal: 229390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Follow-up of TASK-110 (decision-40). TASK-110 moved the settings key rules into descriptors, but the agent-side tables still hold Claude and Codex rows in alz-core: AGENT_KEYS and neutral_keys (item/agent/native.rs), CLAUDE_AGENT_FIELDS and the typed CLAUDE_SETTINGS table. Move them to the descriptor's agent_fields and settings_keys, read through the Registry, with output and every message unchanged.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 grep -rniE 'claude|codex' crates/alz-core/src/item finds no table row of a target
- [ ] #2 Agent native fields and typed settings keys are validated with the target's descriptor through the Registry (tests); codex keeps its validation through its descriptor-only entry
- [ ] #3 mise run rust:ci and mise run alz:check pass with unchanged output
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Implementation (commit 19200e7). Contract (still model 1.0): alz-adapter/native_type.rs holds Shape, Field, Span, Ends (moved from alz-core) and TypedKeys { fields: &'static [Field], tool, recorded_in }; NativeVocabulary.settings_keys and agent_fields are Option<TypedKeys> (None = not typed), plus agent_neutral_keys: Vec<String> and agent_first_class_keys: Vec<FirstClassKey> (path = key, set_through = owner); builders with_settings_keys and with_agent_native; ValueKind, SettingsKey and AgentField removed (nothing used them). Rows now: Claude's typed tables in crates/alz-target-claude/src/typed_keys.rs (git mv of item/native_types/claude.rs), its neutral and first-class agent keys in alz-target-claude/src/descriptor.rs; Codex's neutral and first-class agent keys in the descriptor-only entry alz-core/src/target/codex.rs (no typed keys, so codex stays unchecked for types as before). Reading: AgentItems::read(layer, &Registry) -> NativeFields::read(section, at, registry) -> registry.key_rules(target).vocabulary() (new KeyRules::vocabulary); settings typed_keys reads rules.vocabulary().settings_keys. Agent native sections stay closed to claude and codex (the ids list in NativeFields::read and BASE/OVERLAY_FIELDS); messages unchanged (the 'Claude Code' tool name and 'docs/target-facts.md section 14' come from TypedKeys). A claude map in a registry with no claude adapter is now read as a mapping only, like settings (TASK-110). AC1: grep -rniE 'claude|codex' crates/alz-core/src/item leaves no table row of a target outside tests: native_keys/spec_lists.rs is the cfg(test) spec oracle; the rest are the spec's closed target ids (agent.rs BASE_FIELDS/OVERLAY_FIELDS, NativeFields::read), ItemPath::ClaudeExtra and instruction.rs (CLAUDE.extra.md, spec §7.1), passthrough coverage paths (.claude/...), protected_variable.rs and settings.rs/agent/native.rs calls to env_keys and codex_policies (protected-variable checks, not a table), hook/vocabulary.rs (hook events, descriptor's hook_events is a later move), doc comments. AC2: new tests item/agent/tests.rs::agent_native_fields_are_checked_with_the_descriptor_through_the_registry, ::a_claude_map_of_a_registry_without_a_claude_adapter_is_read_as_a_mapping_and_nothing_more, ::codex_keeps_its_agent_validation_through_its_descriptor_only_entry; item/settings/tests.rs::typed_settings_keys_are_checked_with_the_table_of_the_descriptor, ::a_target_whose_descriptor_types_no_settings_keys_accepts_any_key_its_key_rules_allow; existing assertions unchanged (calls gained &Registry::builtin(); row tests read the tables from the registry's descriptor). AC3: mise run rust:ci exit 0 (fmt, clippy -D warnings, 81 test binaries ok, doc, msrv, deny, audit); mise run alz:check: check passed, 67 generated files match; golden trees and fixtures untouched. docs/spec-coverage.md: new tests added to the 7.2 and 7.9 Typed keys rows (guard passes); docs/target-facts.md section 14 intro and one note name the new table path. Open: Shape tables are &'static, so a WASM adapter (TASK-122) will need an owned form; hook events and protected-variable checks still sit in the core.
<!-- SECTION:NOTES:END -->
