---
id: TASK-55.5.16
title: >-
  R4.16: Security-relevant native settings as floors and reserved keys,
  permission floor fields, no servers or hooks in native fields
status: In Progress
assignee:
  - '@implementer'
created_date: '2026-11-23T16:08:00Z'
updated_date: '2026-11-23T17:40:00Z'
labels:
  - area:rust
  - area:security
  - area:merge
milestone: m-7
dependencies:
  - TASK-55.5.15
parent_task_id: TASK-55.5
priority: high
ordinal: 130390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Owner decisions of 2026-11-23 (TASK-55.5's notes, the architect's approved draft table). In alz-core: (a) a fixed list of security-relevant native settings keys per target with floor semantics — once a layer sets one, a more specific layer may only move it up its stricter order (booleans, ordered enums, 'no additions' lists, 'union' lists); a looser value or a kind change is a settings error naming the layer, file and key path; (b) the reserved keys of the table are settings errors wherever they appear (top level and nested as the table says); (c) permissions.yaml gains first-class floor fields for disableBypassPermissionsMode and blocking reads and writes outside the working directories (once true, stays true), rendered by R5; (d) claude.mcpServers and per-agent hooks in an agent's native fields are agent errors. Each with unit and property tests (a floor key never loosens down the stack), and neutral acceptance rows in test 37 through lint. The architect writes the spec text (§7.6, §7.9, §7.2) with reference drift.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Every floor key of the approved table is refused when a more specific layer loosens it or changes its kind, with a settings error naming the layer, file and key path, and a property test shows no floor key loosens down the stack
- [x] #2 Every reserved key of the table is refused in settings.yaml, and claude.mcpServers and per-agent hooks are refused in an agent's native fields
- [ ] #3 permissions.yaml accepts the new floor fields (once true, stays true), and the spec states every rule of this task with its reference drift
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Implementer (2026-11-23) state: DONE in alz-core: item/native_keys.rs (the one table: KEYS rows with KeyRule FirstClass/Reserved/Floor(Strictness Flag/Ladder/NoAdditions/Union), AGENT_KEYS, loosenings()), item/settings.rs reads reserved keys through the table (generic walk replaced the Codex Level walk), merge/settings.rs apply checks floors (error [settings], file not merged), item/permission.rs + merge/permissions.rs FloorFlag fields (disable_bypass_mode, block_reads_outside, block_writes_outside; false after true = permission-floor error), item/agent/native.rs refuses mcpServers/mcp_servers/hooks (agent error). Unit tests in place. TODO: property tests, test 37 acceptance rows, rust:ci, lint, test, final report.

Implementer (2026-11-23), R4.16 evidence. AC1: crates/alz-core/src/item/native_keys.rs holds the one table (KEYS: NativeKey{target, path with * and ** wildcards, KeyRule::FirstClass|Reserved|Floor(Strictness::Flag|Ladder|NoAdditions|Union)}, plus AGENT_KEYS); loosenings() is the pure check; merge/settings.rs SettingsState::apply runs it against the merged settings of the more general layers and reports [settings] naming layer and file and the table's key path (no value, no profile name); the whole file is then not merged. Unit tests per row type in item/native_keys/tests.rs (boolean both directions, ladder with equal ranks and unknown values, no-additions list and mapping, union, kind change incl. null, free when unset) and merge/settings/tests.rs (through the stack); property no_floor_key_loosens_down_the_stack in merge/settings/laws.rs (mutation-checked: fails without the check). AC2: every FirstClass/Reserved row refused by item/settings.rs (generic walk over the table replaced the Codex Level walk; test every_key_that_the_table_refuses_...); claude.mcpServers, codex.mcp_servers and hooks of either target refused in an agent's and an overlay's native fields as [agent] (item/agent/native.rs, tests in item/agent/tests.rs); the now unreachable server-env check of native fields was removed (ServerField enum gone, server_envs kept for passthrough files). AC3 (code part): permissions.yaml fields disable_bypass_mode, block_reads_outside, block_writes_outside (item::FloorFlag, Permissions::flag); MergedPermissions::flag(FloorFlag)->Option<&Traced<bool>> and is_set(); a later false after true is a [permission-floor] error naming field and both layers and the flag stays true; property a_flag_that_is_true_stays_true_down_the_stack. Acceptance: test/acceptance/37-settings-targets.test.ts new rows (reserved keys, floor keys lint clean, agent native servers/hooks, permissions flags), referenceDrift TASK-55.5.16 where the reference fails them; passes against target/debug/alz, expected-fail against the reference. Floors across layers cannot be shown through lint (it checks one layer alone), so there is no acceptance row for them until R5 build. Checks: mise run rust:ci exit 0 (alz-core 982 tests); mise run lint exit 0; mise run test exit 0 (1185 passed, 171 expected fail). Spec sentences for the architect: 7.9 'A table of native keys with a security meaning lists, per target, the keys that are refused and the keys that are floors. A refused key (a first-class item or a reserved key) in settings.yaml is a settings error naming the file and the key path of the table; a key path may use * for one key and ** for any run of keys, so a Codex features mapping is checked wherever it stands.' 7.9 'A floor key, once a more general layer set it, may only move up its order in a more specific layer: a boolean to its stricter value, an ordered text to the same or a later rank (a value in no rank may not change), a no-additions list or mapping may not gain an entry (a list item or mapping key, at any depth), a union list may only grow; a looser value or a change of kind, null included, is a settings error naming the layer, the file and the key path, and none of that file's settings are merged. This departs from the left fold for these keys only. A key that no more general layer set or set to null is free.' 7.9 'The list is the owner-approved table of TASK-55.5 (2026-11-23); every key in it needs a docs/target-facts.md entry before an adapter relies on it.' 7.2 'claude.mcpServers, codex.mcp_servers and hooks (either target) in an agent's or overlay's native fields are an agent error naming the file and the key, since servers and hooks come only through mcp.yaml and hooks.yaml.' 7.6 'permissions.yaml may set disable_bypass_mode, block_reads_outside and block_writes_outside to true or false; once any layer sets one to true it stays true, and a more specific layer that sets it to false is a permission-floor error naming the field and both layers.' Reference drift: the reference refuses the three fields as unknown, accepts the new reserved keys and accepts mcpServers/hooks in agent native fields; reference never checks floors. Open: false after true being an error (not silently ignored) is my reading of 'once true, stays true'; the architect may prefer a silent join. Plugin keys are reserved with the reason 'brings hooks and servers past the allowlists'.

Verified by the lead (2026-11-23): mise run rust:ci exits 0, lint passes, src/ unchanged. Lead: a later false after true for the permission floor fields is a permission-floor error (as removing a deny is); sandbox.credentials reserved as a whole key accepted. AC #3's code part is done; its spec part comes in the next spec pass.
<!-- SECTION:NOTES:END -->
