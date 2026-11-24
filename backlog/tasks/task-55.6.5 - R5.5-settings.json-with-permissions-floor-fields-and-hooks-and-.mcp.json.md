---
id: TASK-55.6.5
title: 'R5.5: settings.json with permissions, floor fields and hooks, and .mcp.json'
status: Done
assignee:
  - '@implementer'
created_date: '2026-11-23T18:08:00Z'
updated_date: '2026-11-24T02:19:00Z'
labels:
  - area:rust
  - area:render
  - area:security
milestone: m-7
dependencies:
  - TASK-55.6.2
  - TASK-76
parent_task_id: TASK-55.6
priority: high
type: task
ordinal: 137390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Plan for this step (§7.5 to §7.9, §8.1, target-facts §4, §7b, §7c).
- permissions: allow, ask and deny as Claude rules, defaultMode, additionalDirectories from workspace (hub and add_dirs as written), and the three floor fields as permissions.disableBypassPermissionsMode, blockReadsOutsideWorkingDirectories and blockWritesOutsideWorkingDirectories, only once docs/target-facts.md records them (TASK-76).
- hooks: grouped by native event in stack and file order; one handler per tool and path condition; `run:` copies the script once to `.claude/hooks/<layer>/<path>` with its executable bit and the command `"${CLAUDE_PROJECT_DIR}/<path>"`; `command:` with the project-dir variable rewritten; matcher, timeout and onFailure as the reference writes them, with the hook-on-fail warnings.
- model: the default alias's Claude model.
- The generated keys deep-merged over Merged::settings for claude, written with the stable JSON emitter (keys in code-point order, decision-30); no settings.json when the result is empty.
- .mcp.json: the enabled servers (stdio: command, args, env as `${NAME}` references; http and sse: type and url), only when one is enabled.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 settings.json, the copied hook scripts and .mcp.json match the reference's bytes for fixtures 07, 08, 09, 16, 19 and 33, keys in code-point order (Rust tests)
- [x] #2 The floor fields are written under the three permission keys exactly when a layer set them true, and no other path writes those keys (unit tests)
- [x] #3 on_fail: deny on stop or subagent_stop and on post_tool gives the hook-on-fail warnings, and a run: script used by two hooks is copied once (unit tests)
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
R5.5 implementation (implementer, 2026-11-24), uncommitted on feat/rust-r5-task-55.6.
Shape: alz-core target/claude/{settings,hooks,permissions,mcp,document}.rs; public entry ClaudeSettingsFiles::of(&Merged, &Workspace) -> tree + diagnostics (same shape as ClaudeFiles; both share a private assemble()). The manifest is not in Merged, so the caller (R5.6) passes manifest.workspace(); R5.6 merges the two trees (no path overlap: .claude/settings.json, .claude/hooks/**, .mcp.json). Also: Code::HookOnFail (hook-on-fail), tools::rule (Claude rule text), HookScript now holds the script's bytes and executable bit (read from LayerContents at hook read time), merge/testing::layer_executable.
AC1: goldens made once from dist/cli.js (reference) in a scratch copy of each fixture, in crates/alz-core/tests/fixtures/claude-settings/ (README there). Byte-identical, paths and mode, for fixtures 01, 07 (repo-combine), 08 (repo-unlocked), 09, 16 (repo-a, hub), 19, plus a stack written for the test (mix: every event, a script two hooks run, codex-only hooks, sse/http/stdio, env_from, default model, settings.yaml keys, workspace dirs, one executable and one plain script). 08 repo-locked is the hook-locked error, so it has no output. 33's case is JSON key order: unit test settings_keys_are_in_code_point_order_whole_numbers_included (U+FF61 before U+1F600, 10 before 9). Tests: target::claude::tests::settings::*.
AC2: permissions.rs is the only writer of the three floor keys (settings.yaml cannot set permissions: merge refuses it, test settings_yaml_cannot_write_a_permission_key); disableBypassPermissionsMode is the string disable, the two block keys are booleans; proptest the_floor_keys_are_written_exactly_for_the_fields_a_layer_set over all 8 flag subsets.
AC3: deny_on_stop_subagent_stop_and_post_tool_gives_the_hook_on_fail_warnings (exact lines), a_script_two_hooks_run_is_copied_once, proptest rendering_is_deterministic_and_every_output_is_a_path_of_the_tree (determinism; output paths = settings.json when any hook plus exactly the scripts used).
Checks: mise run rust:ci green (alz-core lib 1116 tests); mise run lint green; mise run test green (50 files, 1186 passed, 188 expected fail, 5 skipped). biome.json now excludes crates/*/tests/fixtures (goldens must stay byte-exact; json goldens failed biome format).
Open: (1) .mcp.json omits args when empty; the reference writes args: [] only when the layer wrote an explicit empty list, which the Rust model cannot tell from none (spec §8.1 could say so). (2) the default model alias is written as given, with no is_model_value check, as the reference does. (3) ClaudeSettingsFiles is not yet wired into Rendered (R5.6).

Parity fix (coordinator, decision-30): Transport::Stdio.args is now Option<Vec<String>> (None when args is not written; schema::written_text_list), and .mcp.json writes args: [] exactly when the layer wrote an empty list. Reference output for the mix stack now holds server 'empty' (args: []) next to 'plain' (no args); test args_written_empty_are_written_and_absent_args_are_left_out_like_the_reference plus the mix golden. rust:ci (alz-core lib 1117 tests) and lint green. Open item (1) above is closed.

Verified by the lead (2026-11-24): mise run rust:ci and mise run lint exit 0, src/ unchanged, goldens hold no machine paths. Lead decision applied: .mcp.json writes args: [] exactly when the layer wrote an empty list (parity). biome.json excludes crates/*/tests/fixtures so the byte-exact goldens are not reformatted.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
ClaudeSettingsFiles::of renders .claude/settings.json (merged target settings, permission rules, defaultMode, additionalDirectories, the floor keys only from permissions.yaml, hooks with their scripts copied once with their mode) and .mcp.json (servers sorted by id), byte for byte as the reference does for fixtures 01, 07, 08, 09, 16 and 19 and a mixed stack.
<!-- SECTION:FINAL_SUMMARY:END -->
