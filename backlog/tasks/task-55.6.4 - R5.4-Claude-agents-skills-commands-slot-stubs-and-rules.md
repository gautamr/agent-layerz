---
id: TASK-55.6.4
title: 'R5.4: Claude agents, skills, commands, slot stubs and rules'
status: Done
assignee:
  - '@implementer'
created_date: '2026-11-23T18:08:00Z'
updated_date: '2026-11-24T00:01:00Z'
labels:
  - area:rust
  - area:render
milestone: m-7
dependencies:
  - TASK-55.6.2
  - TASK-55.6.17
parent_task_id: TASK-55.6
priority: high
type: task
ordinal: 136390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Plan for this step (§7.2 to §7.4, §7.10, §8.1, target-facts §1, §5, §9), in alz-core target/claude/.
- Agents: `.claude/agents/<name>.md` for agents that target Claude; frontmatter name, description, tools (neutral to native names, deduplicated, joined with `, `; a scoped tool warns `tool-scope`; an empty list is the error claude-agent), model through the routing (an id that is not a Claude model value is claude-agent), permissionMode, skills, isolation and memory as the reference writes them, then the native fields in the order NativeMap holds them (TASK-66: whole-number keys keep source order); the header as a YAML comment; the body.
- Skills: every file of the winning skill directory, SKILL.md with the header comment inserted, other files byte for byte with their executable bit. Commands: a SKILL.md with name, description, argument-hint and disable-model-invocation: true. Slot stubs: the stub text of the reference.
- Rules: `.claude/rules/<name>.md` with `paths:` frontmatter and the header.
All YAML through the R2.8 emitter.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Every agent, skill, command, stub and rule file of fixtures 03 to 06, 17 and 18 matches the reference's bytes (Rust tests)
- [x] #2 A generated agent whose native fields hold the keys 10 and 9 after the fixed fields writes them in source order; the acceptance case is in the suite, marked referenceDrift TASK-66 (TASK-66 AC #2)
- [x] #3 An empty tools list and a non-Claude model id are the error claude-agent, and a scoped tool gives the warning tool-scope, each naming the agent and its file (unit tests)
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
From R5.17 (2026-11-24): the reference refuses an agent overlay list string that is only white space ('cannot parse'); the Rust reader accepts it as an edit with no segment. Refuse it as the reference does (stricter, and the same output), with a test.

From R5.3 (2026-11-24): merge/agents.rs prints the layer kind raw in the agent overlay heading; print it stripped as layer/heading.rs LayerHeading now does (spec §7.1, drift TASK-55.6.3).

Implementer state (2026-11-24, partial, uncommitted): DONE: (1) merge/agents.rs ContextSection prints the layer kind stripped + test; (2) item/list_edit.rs segments refuses a non-empty white-space-only string (empty string still an edit with no segment), tests in item/agent/tests.rs; two test-side generators (item/agent writing_edits..., merge/permissions/laws edit_text) no longer build ' ' for empty parts; reference permissions reader shares parseListOps so parity holds; (3) Code::ClaudeAgent and Code::ToolScope; (4) render/sources.rs (refs_of, source_error moved out of instructions.rs); (5) LayerContents carries the owner-execute bit (with_executable, is_executable) and SkillFile::executable; alz-io must fill it from the walk (fs/scan.rs read_file already knows it; also needed by R5.5 hook scripts and R5.6 passthrough); (6) goldens from the reference in crates/alz-core/tests/fixtures/claude-files/{01,03,04,05,06,17,18,22}. TODO: target/claude/{agents,skills,rules,tools}.rs, tests, spec lines (native null/float/out-of-range ints refused as claude-agent because the emitter writes none, spec 10.2; models error/warning for agent model routing), rust:ci, lint.

Implementer result (2026-11-24, uncommitted, rust:ci exit 0, lint exit 0). Code: alz-core src/target/claude.rs (ClaudeFiles::of(&Merged) -> FileTree + Diagnostics, pure) with claude/{agents,skills,rules,frontmatter,tools}.rs; render/sources.rs (shared header sources); Code::ClaudeAgent, Code::ToolScope; LayerContents::with_executable/is_executable and SkillFile::executable; ContextSection prints the layer kind stripped; item/list_edit.rs refuses a non-empty white-space-only list string ('' stays an edit with no segment; permissions share the reader, as parseListOps does in src/). Evidence per AC: #1 target/claude/tests.rs renders the merged stack of fixtures 01, 03, 04 (repo-narrow), 05, 06 (repo-loose, slot stub), 17, 18 (repo), 22 and compares every .claude/agents, skills and rules file byte for byte with goldens made once from the reference's alz build (crates/alz-core/tests/fixtures/claude-files/<fixture>/.claude/..., no machine paths: grep clean) and asserts the tree holds exactly those paths; fixture 03 also matches test/fixtures/03-agent-overlay/expected. Commands (01 learn) and the stub (06) are in those goldens. #2 unit test native_fields_hold_the_keys_10_and_9_in_source_order_after_the_fixed_fields; acceptance case in test/acceptance/18-native-fields.test.ts (fixture repo-key-order) marked referenceDrift TASK-66; it fails against the reference for the right reason (reference writes "9" and "10" before name) and runs as expected-fail (vitest: 2 passed, 1 expected fail); it passes against ALZ_BIN only once build exists (R5.8). #3 unit tests an_empty_tools_list_and_a_non_claude_model_are_the_error_claude_agent, tools_are_native_names_without_duplicates_and_a_scope_is_a_warning (and the tool-scope warning text in fixture_03). Property test rendering_is_deterministic_and_every_output_is_a_path_of_the_tree. LEFT OPEN: (a) alz-io must fill the owner-execute bit of layer files into LayerContents::with_executable from the walk (alz-io/src/fs/scan.rs read_file already knows it); until then skill scripts render Regular; R5.5 hook scripts and R5.6 passthrough need the same bit; the Rust unit test proves the render side. (b) The 04 repo-widen-* goldens were not made (reference drift TASK-55.5.6). (c) Spec: s8.1 now says a native null/float/integer beyond 64 bits is the error claude-agent (emitter writes none) and states the models error/warning of agent model routing; s10.1 row updated; architect to review. (d) Code::Models message wording for an unknown agent alias follows the reference.

Verified by the lead (2026-11-24): mise run rust:ci and mise run lint exit 0, src/ unchanged, goldens hold no machine paths. Accepted: .gitignore's target/ became /target/ (it hid crates/alz-core/src/target/); the §8.1 sentences (a native null, float or integer beyond 64 bits is error claude-agent, reference drift; the model alias rule as the reference has it). TASK-66's acceptance case passes against the Rust binary once alz build exists (R5.8).
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
ClaudeFiles::of renders .claude/agents, skills, commands, slot stubs and rules from the merged model, byte for byte as the reference does for fixtures 01, 03, 04, 05, 06, 17, 18 and 22; native fields keep source order after the fixed fields (TASK-66); agent diagnostics name the agent and its file; rendering is deterministic.
<!-- SECTION:FINAL_SUMMARY:END -->
