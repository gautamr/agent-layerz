---
id: TASK-55.5.10
title: >-
  R4.10: Settings with known targets, and protected variables in settings and
  native fields
status: To Do
assignee:
  - '@implementer'
created_date: '2026-11-23T12:27:00Z'
updated_date: '2026-11-23T12:39:00Z'
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
- [ ] #1 A settings.yaml whose top-level key is not a known target is refused with a settings error naming the file, whatever the key (table test of keys), and the merged settings are an ordered map keyed by TargetName
- [ ] #2 The reserved keys of each target are refused naming the file that sets them, and the deep merge follows §7.9: a later kind replaces and list items equal as ordered trees appear once (unit tests), associative over kind-consistent trees (proptest)
- [ ] #3 Every protected-variable case of §7.9 for settings.yaml and native agent fields is refused with code protected-variable, names no value, and matches names in any ASCII case (unit tests and a proptest over case variants)
<!-- AC:END -->
