---
id: TASK-55.6.13
title: 'R5.13: Adoption planning'
status: Done
assignee:
  - '@implementer'
created_date: '2026-11-23T18:09:00Z'
updated_date: '2026-11-24T06:22:00Z'
labels:
  - area:rust
  - area:adopt
  - area:security
milestone: m-7
dependencies:
  - TASK-55.6.6
  - TASK-55.6.1
parent_task_id: TASK-55.6
priority: high
type: task
ordinal: 145390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Plan for this step (§4.3), pure in alz-core adopt/.
- Inputs: the collisions and the adopt-always files (.claude/settings.json, .mcp.json) that alz did not generate, read by RepoRoot as bytes; the Merged stack before adoption; the repo layer's existing files.
- AdoptionPlan: layer files to write (YAML through the R2.8 emitter, merged into existing layer YAML with maps deep and lists appended), moves, removals, manifest additions (workspace.add_dirs, mcp.enable), the report, notes and problems. AGENTS.md to instructions/00-adopted.md; CLAUDE.md to targets/claude/CLAUDE.extra.md without @AGENTS.md and without paragraphs AGENTS.md holds; agents to neutral agents (native-only fields under claude:, models as aliases, an agent a general layer defines as a mode: replace overlay); skills and rules moved; hook scripts to hooks/; settings.json to permissions.yaml, hooks/hooks.yaml, settings.yaml, the models default and add_dirs; .mcp.json to mcp.yaml and mcp.enable.
- TASK-73: the planned repo layer is read in memory with the R4 per-layer readers before the plan is accepted; any text the reader would refuse is a problem naming the source file, never the text.
- JSON is read with the exactness owner question 4 decides. Problems print untrusted text stripped (class 1).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Adopting fixture 20 plans the layer files, moves and manifest additions the reference makes, byte for byte for the layer files (unit tests)
- [x] #2 A literal env value in .mcp.json, an unknown permission tool, a hook type other than command, or text the layer reader would refuse is a problem, and the plan is then rejected whole (unit tests, TASK-73)
- [x] #3 Building the adopted layer gives the settings, hooks and agents of the native files it adopted (property over generated native settings, test 20's law)
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
2026-11-24 implementer: crates/alz-core/src/adopt.rs and adopt/ (plan(), AdoptionInput, AdoptionPlan, AdoptionRefused, NativeFile; draft, values, names, claude_md, agent, settings_json, hooks, mcp_json, manifest_edit, check, tests). Pure: natives arrive as observed bytes. Evidence AC#1: unit test adopting_fixture_20_plans_the_layer_files_the_reference_writes compares permissions.yaml, hooks/hooks.yaml, settings.yaml, models.yaml, mcp.yaml, agents/*.md and CLAUDE.extra.md byte for byte with what the reference wrote when run once on a scratch copy of fixture 20; adopting_fixture_20_moves_files... pins moves, removals and the manifest edit (keeps the comment; the reference rewrote its spacing). Drift found by the reader check, not reproduced: (a) the reference moved hook scripts to hooks/<path> and wrote run: <path>, which alz's reader refuses (run: must name hooks/scripts/); the plan moves to hooks/scripts/<path> and writes run: scripts/<path>; spec §4.3 table updated. (b) fixture 20's architect agent carries native hooks, which the Rust agent rule refuses (claude.hooks is set through hooks/hooks.yaml), so adopting fixture 20 whole is refused in Rust with a problem naming .claude/agents/architect.md (test an_agent_with_native_hooks_is_refused...). R5.15 needs a decision: change the fixture or define how an agent's native hooks are adopted. Floats, nulls and integers beyond 64 bits in settings.json or .mcp.json are a problem (the emitter of 10.2 writes none). serde_json float_roundtrip enabled in Cargo.toml and removed from the deny.toml ban list; Code::Adopt added. AC#2 evidence: tests a_literal_env_value_in_mcp_json_rejects_the_plan_without_printing_it, an_unknown_permission_tool_and_a_hook_type_other_than_command_are_problems, text_the_layer_reader_refuses_rejects_the_plan_and_the_problem_names_the_native_file (U+2028), a_settings_key_the_typed_table_refuses_is_not_planned (TASK-79). AC#3 (property over generated native settings, built through the renderer) is NOT done here: open for R5.14/R5.15 where the renderer is wired; the reader check (merge of the planned layer) is done here.

Verified by the lead (2026-11-24): merged into the R5 branch (2640bd2); mise run rust:ci and mise run lint exit 0; serde_json float_roundtrip enabled per decision-33's amendment. AC #3 moves to R5.14, where render is wired (noted there).

AC #3 met as R5.14's AC #4 (2026-11-24): the proptest rendered_native_settings_equal_the_adopted_ones in crates/alz/tests/adopt.rs.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
adopt::plan turns the native files alz did not generate into a planned repo layer (layer files, moves, removals, a comment-keeping layer.yaml edit) and runs it through the real readers and merge, so a plan never holds text the reader would refuse; refused plans name the file and never print values. Fixture 20's files match the reference byte for byte.
<!-- SECTION:FINAL_SUMMARY:END -->
