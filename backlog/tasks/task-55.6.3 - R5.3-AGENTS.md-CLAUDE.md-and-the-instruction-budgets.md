---
id: TASK-55.6.3
title: 'R5.3: AGENTS.md, CLAUDE.md and the instruction budgets'
status: Done
assignee:
  - '@implementer'
created_date: '2026-11-23T18:08:00Z'
updated_date: '2026-11-23T23:31:00Z'
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
ordinal: 135390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Plan for this step (§7.1, §7.12, §8.1).
- AGENTS.md from Merged::instructions: the header with every block's source ref (and the declaring layer's layer.yaml when a commit identity is declared; `no instructions` when there is none), one `## <Kind>: <name>` heading per layer run (kind with its first character in upper case), the blocks, then the §7.12 Commits section from the winner of Merged::commit_identity.
- The budget check of the composed text, Commits section included: instructions_max_lines (lines counted as the reference does) and instructions_max_bytes, each an error `budget`.
- CLAUDE.md: header naming AGENTS.md and each CLAUDE.extra.md, `@AGENTS.md`, then `## Claude-specific` and the extras in stack order.
Byte parity with the reference for fixtures 01, 02, 16 and 26.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 AGENTS.md and CLAUDE.md match the reference's bytes for fixtures 01, 02, 16 and 26, Commits section included (Rust tests over Merged values built from those fixtures)
- [x] #2 A composed AGENTS.md over instructions_max_lines or instructions_max_bytes is the error budget, and the Commits section counts toward both (unit tests at the limit and one over it)
- [x] #3 Rendering is deterministic: render(m) == render(m) byte for byte for generated stacks (proptest)
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Implementer (2026-11-24), R5.3 done in alz-core, not committed.
Code: render/instructions.rs (AgentsMd::compose, AgentsMd::budget_problems, claude_md, agents_md_path, claude_md_path); Code::Budget; SourceRef::NoInstructions and SourceRef/LayerLabel::of_origin (header.rs); InstructionBlock::origin and MergedClaudeExtra (merge/instructions.rs, Merged::claude_extras now returns them, each with its Origin and revision); Declared::Winner holds an Origin (of layer.yaml) so the Commits source ref carries the declarer's revision (Declared::origin()); Origin::layer_yaml; LayerHeading prints the kind stripped of control and format characters (spec 7.1 sentence added, reference drift).
AC1: tests render each fixture's merged stack and compare byte for byte: 02 against test/fixtures/02-instructions/expected/{AGENTS,CLAUDE}.md; 01, 16 (repo-a, repo-b, hub label acme@layer-v0.1.0) and 26 (client Commits section, source client@0.1.0:layer.yaml last) against goldens made once with the reference's alz build, in crates/alz-core/tests/fixtures/instructions/ (these fixtures have no expected/ folder). Also checked against the reference: a stack with no instructions (header 'no instructions').
AC2: unit tests at the limit (ok) and one over (error [budget] with the spec text, lines and bytes), and a test that the text without the Commits section just meets a limit the full text exceeds, for both budgets.
AC3: proptest rendering_is_deterministic_for_generated_stacks.
Checks: mise run rust:ci exit 0 (1047 alz-core tests incl. 15 new), mise run lint ok, mise run test 1185 passed.
Open for R5.6: the caller adds budget_problems(manifest.budgets()) to the build diagnostics; compose errors are error [layer] at <layer>:<path>. merge/agents.rs:74 still prints the kind raw in the agent overlay heading (R5.4 should print it stripped like LayerHeading).

Verified by the lead (2026-11-24): mise run rust:ci exits 0, src/ unchanged, goldens hold no machine paths. The §7.1 sentence (layer kind written stripped, reference drift) is accepted.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
AGENTS.md and CLAUDE.md render from the merged model byte for byte as the reference does for fixtures 01, 02, 16 (both repositories) and 26 (with the Commits section), headers carrying each source's revision; the line and byte budgets give error budget; rendering is deterministic.
<!-- SECTION:FINAL_SUMMARY:END -->
