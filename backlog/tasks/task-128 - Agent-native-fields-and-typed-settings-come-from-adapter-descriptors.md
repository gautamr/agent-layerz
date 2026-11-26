---
id: TASK-128
title: Agent native fields and typed settings come from adapter descriptors
status: Done
assignee: []
created_date: '2026-11-26T07:58:00Z'
updated_date: '2026-11-26T09:12:00Z'
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
- [x] #1 grep -rniE 'claude|codex' crates/alz-core/src/item finds no table row of a target
- [x] #2 Agent native fields and typed settings keys are validated with the target's descriptor through the Registry (tests); codex keeps its validation through its descriptor-only entry
- [x] #3 mise run rust:ci and mise run alz:check pass with unchanged output
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Agent native fields, neutral and first-class agent keys and typed settings come from each adapter's descriptor (Claude in alz-target-claude, codex through its descriptor-only entry) through the Registry; alz-core/src/item holds no target table row. Output and messages unchanged. Merged in #50.
<!-- SECTION:FINAL_SUMMARY:END -->
